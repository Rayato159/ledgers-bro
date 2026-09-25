use crate::{components::*, overview::thai_month};
use dioxus::prelude::*;
use ledger_application::{
    Dashboard, FlowHealth, monthly_cashflow, projected_cashflow, recurring_month,
};

#[component]
pub(crate) fn CashflowChart(view: Dashboard) -> Element {
    let mut detail_tab = use_signal(|| None::<usize>);
    let mut selected = use_signal(|| 5_usize);
    let mut include_pending = use_signal(|| true);
    let mut store = use_context::<crate::state::UiState>();
    let periods = match monthly_cashflow(&view).and_then(|periods| {
        if include_pending() {
            periods
                .iter()
                .map(|period| projected_cashflow(&view, period))
                .collect()
        } else {
            Ok(periods)
        }
    }) {
        Ok(periods) => periods,
        Err(_) => {
            return rsx! { section { class: "card", role: "alert", {crate::i18n::text("ยอดรวมเกินขอบเขตที่แสดงได้ กรุณาตรวจรายการ", &[])} } };
        }
    };
    let period = &periods[selected().min(5)];
    let pending = ledger_domain::Month::new(period.year, period.month)
        .and_then(|month| recurring_month(&view, month))
        .map(|r| money_label(r.pending))
        .unwrap_or_else(|_| "—".into());
    let (health, class, advice) = match period.health() {
        FlowHealth::NoData => ("ยังไม่มีข้อมูล", "empty", "เริ่มบันทึกรายรับและรายจ่ายเพื่อดูแนวโน้ม"),
        FlowHealth::Poor => ("แย่", "poor", "รายจ่ายมากกว่ารายรับ ลองทบทวนรายจ่ายที่ลดได้"),
        FlowHealth::Fair => (
            "พอใช้",
            "fair",
            "รายรับพอจ่าย แต่ส่วนต่างยังน้อย ลองเพิ่มเงินเหลือในแต่ละเดือน",
        ),
        FlowHealth::Good => ("ดี", "good", "รายรับมากกว่ารายจ่าย มีส่วนต่างสำหรับออมหรือสำรอง"),
    };
    let net = period
        .income
        .checked_sub(period.expenses)
        .map(money_label)
        .unwrap_or_else(|_| "—".into());
    let rate = period.rate();
    let rate_label = rate
        .map(|rate| {
            if rate == 0 && period.income != period.expenses {
                if period.income < period.expenses {
                    "−0.0001 < rate < 0".into()
                } else {
                    "0 < rate < +0.0001".into()
                }
            } else {
                format!("{:+.4}", f64::from(rate) / 10_000.0)
            }
        })
        .unwrap_or_else(|| "—".into());
    let position = rate
        .map(|rate| f64::from(rate + 10_000) / 200.0)
        .unwrap_or(50.0);
    let income_share = period
        .income_share()
        .map(|share| f64::from(share) / 100.0)
        .unwrap_or(0.0);
    let expense_share = if rate.is_some() {
        100.0 - income_share
    } else {
        0.0
    };
    let comparison = if rate.is_none() {
        crate::i18n::tr("ยังไม่มีรายรับหรือรายจ่ายในเดือนนี้")
    } else if period.income == period.expenses {
        crate::i18n::tr("รายรับเท่ากับรายจ่าย · ต่างกัน 0%")
    } else {
        let (larger, smaller) = if period.income > period.expenses {
            ("รายรับ", "รายจ่าย")
        } else {
            ("รายจ่าย", "รายรับ")
        };
        match period.difference_percent() {
            Some(percent) => crate::i18n::text(
                "{0}มากกว่า{1} {2}% (เทียบกับ{1})",
                &[
                    crate::i18n::tr(larger),
                    crate::i18n::tr(smaller),
                    format!("{}.{:02}", percent / 100, percent % 100),
                ],
            ),
            None => crate::i18n::text(
                "มีเฉพาะ{0} · {1}เป็นศูนย์ จึงเทียบเป็นเปอร์เซ็นต์ไม่ได้",
                &[crate::i18n::tr(larger), crate::i18n::tr(smaller)],
            ),
        }
    };
    let maximum = periods
        .iter()
        .map(|p| p.income.minor().max(p.expenses.minor()))
        .max()
        .unwrap_or(0)
        .max(1);
    rsx! {
        section { class: "card flow-card", "aria-labelledby": "flow-title",
            div { class: "section-heading", div { h2 { id: "flow-title", {crate::i18n::text("รายรับเทียบรายจ่ายที่บันทึก", &[])} } p { class: "muted small", {crate::i18n::text("ย้อนหลัง 6 เดือน · แตะเดือนเพื่อดูรายละเอียด", &[])} } }
                span { class: "flow-health {class}", "{crate::i18n::tr(&health)}" }
            }
            label { class: "flow-mode", input { r#type: "checkbox", checked: include_pending(), onchange: move |event| include_pending.set(event.checked()) } {crate::i18n::text("รวมรายจ่ายประจำที่ยังค้างจ่าย", &[])} }
            p { class: "field-hint", if include_pending() { {crate::i18n::text("กำลังแสดงยอดบันทึกจริง + ภาระประจำที่ยังไม่จ่ายของแต่ละเดือน · รายรับใช้เฉพาะที่บันทึกแล้ว", &[])} } else { {crate::i18n::text("กำลังแสดงเฉพาะรายรับ–รายจ่ายที่บันทึกจริง", &[])} } }
            div { class: "flow-layout",
                div {
                    div { class: "flow-legend", span { class: "flow-income-key", {crate::i18n::text("รายรับ", &[])} } span { class: "flow-expense-key", {crate::i18n::text("รายจ่าย", &[])} } }
                    div { class: "flow-bars", role: "group", "aria-label": crate::i18n::text("กราฟรายรับและรายจ่ายรายเดือน", &[]),
                        for (index, month) in periods.iter().enumerate() {
                            { let incoming = month.income.minor() as f64 / maximum as f64 * 100.0;
                              let outgoing = month.expenses.minor() as f64 / maximum as f64 * 100.0;
                              rsx! {
                                button { r#type: "button", class: if index == selected() { "flow-month selected" } else { "flow-month" }, "aria-pressed": index == selected(),
                                    "aria-label": crate::i18n::text("{0} {1} รายรับ {2} บาท รายจ่าย {3} บาท", &[thai_month(month.month).to_string(), format!("{}", crate::i18n::year(month.year)), money_label(month.income).to_string(), money_label(month.expenses).to_string()]),
                                    onclick: move |_| selected.set(index),
                                    span { class: "flow-bar-pair", "aria-hidden": "true",
                                        span { class: "flow-bar incoming", style: "height: {incoming}%" }
                                        span { class: "flow-bar outgoing", style: "height: {outgoing}%" }
                                    }
                                    span { class: "flow-month-label", "{month.month}/{(crate::i18n::year(month.year)) % 100}" }
                                }
                              }
                            }
                        }
                    }
                    p { class: "field-hint", {crate::i18n::text("ยอดจริงนับตามวันที่รายการ ถึงวันที่ {0} · ไม่รวมยอดเริ่มต้น เงินโอน และรายการที่ยกเลิก", &[format!("{}", view.today)])} if include_pending() { {crate::i18n::text(" · แผนค้างจ่ายรวมทั้งเดือน", &[])} } }
                }
                div { class: "flow-details", "aria-live": "polite",
                    h3 { "{thai_month(period.month)} {crate::i18n::year(period.year)}" }
                    div { class: "flow-amounts",
                        button { class: "flow-amount-button", "aria-haspopup": "dialog", onclick: move |_| detail_tab.set(Some(2)), small { {crate::i18n::text("รายรับ", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(period.income)}" } span { {crate::i18n::text("{0}% ของยอดรวม", &[format!("{:.1}", income_share)])} } small { {crate::i18n::tr("ดูรายการย่อย")} " →" } }
                        button { class: "flow-amount-button", "aria-haspopup": "dialog", onclick: move |_| detail_tab.set(Some(0)), small { {crate::i18n::text("รายจ่าย", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(period.expenses)}" } span { {crate::i18n::text("{0}% ของยอดรวม", &[format!("{:.1}", expense_share)])} } small { {crate::i18n::tr("ดูรายการย่อย")} " →" } }
                    }
                    p { "{comparison}" }
                    button { class: "text-button", "aria-haspopup": "dialog", onclick: move |_| detail_tab.set(Some(1)), {crate::i18n::text("รายจ่ายประจำค้างจ่าย ฿{0}", std::slice::from_ref(&pending))} " →" }
                    button { class: "text-button", onclick: move |_| store.page.set(crate::state::Page::Recurring), {crate::i18n::text("จัดการรายจ่ายประจำ →", &[])} }
                    p { class: "flow-net", {crate::i18n::text("เงินเหลือสุทธิ ฿{0}", std::slice::from_ref(&net))} }
                    div { class: "flow-rate-heading", span { "Flow rate" } strong { "{rate_label}" } }
                    div { class: "flow-gauge", role: "img", "aria-label": crate::i18n::text("Flow rate {0} ช่วงลบหนึ่งถึงบวกหนึ่ง", std::slice::from_ref(&rate_label)),
                        if rate.is_some() { span { class: "flow-pointer", style: "left: {position}%" } }
                    }
                    div { class: "flow-scale", span { "−1.0" } span { "0" } span { "+1.0" } }
                    p { "{crate::i18n::tr(&advice)}" }
                }
            }
            details { class: "flow-explanation", summary { {crate::i18n::text("สูตรและเกณฑ์สุขภาพการเงิน", &[])} }
                p { {crate::i18n::text("Flow rate = (รายรับ − รายจ่าย) ÷ (รายรับ + รายจ่าย) มีค่าตั้งแต่ −1 ถึง +1 ถ้าทั้งคู่เป็นศูนย์จะแสดงว่ายังไม่มีข้อมูล", &[])} }
                p { {crate::i18n::text("แย่: ต่ำกว่า 0 · พอใช้: ตั้งแต่ 0 แต่น้อยกว่า 0.10 · ดี: ตั้งแต่ 0.10 ขึ้นไป เป็นเกณฑ์ของแอปตามโหมดกราฟที่เลือก ไม่ได้ประเมินหนี้ทั้งหมดหรือเงินสำรอง", &[])} }
                p { {crate::i18n::text("ตัวอย่าง รายรับ 30,000 รายจ่าย 20,000 → Flow rate +0.200 และรายรับมากกว่ารายจ่าย 50% การรูดบัตรนับเป็นรายจ่ายวันที่รูด ส่วนโอนจ่ายบัตรไม่นับซ้ำ", &[])} }
            }
        }
        if let Some(initial) = detail_tab() { if let Ok(month) = ledger_domain::Month::new(period.year, period.month) {
            CashflowDetailsDialog { view: view.clone(), month, initial, include_pending: include_pending(), category: None, description: None, onclose: move |_| detail_tab.set(None) }
        } }
    }
}

#[component]
pub(crate) fn CashflowDetailsDialog(
    view: Dashboard,
    month: ledger_domain::Month,
    initial: usize,
    include_pending: bool,
    category: Option<ledger_domain::Category>,
    description: Option<String>,
    onclose: EventHandler,
) -> Element {
    let tab = use_signal(|| initial);
    let breakdown = ledger_application::cashflow_breakdown(&view, month);
    rsx! {
        dialog { id: "flow-breakdown-dialog", class: "account-dialog flow-breakdown-dialog", "aria-labelledby": "flow-breakdown-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('flow-breakdown-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); onclose.call(()); },
            div { class: "section-heading", h2 { id: "flow-breakdown-title", {crate::i18n::tr("รายละเอียดรายรับและรายจ่าย")} " · {month}" }
                button { class: "icon-button", "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| onclose.call(()), Icon { name: "close", size: 20 } }
            }
            match breakdown {
                Err(error) => rsx! { p { role: "alert", "{error}" } },
                Ok(details) => rsx! {
                    if category.is_none() {
                        div { class: "flow-reconciliation",
                            p { {crate::i18n::tr("รายรับ")} strong { "{crate::i18n::currency_prefix()}{money_label(details.recorded.income)}" } }
                            p { {crate::i18n::tr("รายจ่ายที่บันทึกแล้ว")} strong { "{crate::i18n::currency_prefix()}{money_label(details.recorded.expenses)}" } }
                            p { {crate::i18n::tr(if include_pending { "บิลค้างที่รวมในกราฟ" } else { "บิลค้างที่ไม่รวมในกราฟ" })} strong { "{crate::i18n::currency_prefix()}{money_label(details.pending_total)}" } }
                            p { {crate::i18n::tr("รายจ่ายรวมในกราฟ")} strong { {details.recorded.expenses.checked_add(if include_pending { details.pending_total } else { ledger_domain::Money::ZERO }).map(|v| format!("{}{}", crate::i18n::currency_prefix(), money_label(v))).unwrap_or_else(|_| "—".into())} } }
                        }
                        PageTabs { id: "flow-breakdown", tabs: vec![("up", "รายจ่ายที่บันทึกแล้ว"), ("calendar", "บิลค้าง"), ("down", "รายรับ")], selected: tab }
                    } else if let Some(category) = category { h3 { {description.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| crate::i18n::tr(category.label()))} } }
                    for index in 0..3 { PagePanel { id: "flow-breakdown", index, selected: tab(),
                        if index == 1 {
                            if details.pending_items.is_empty() { p { class: "muted", {crate::i18n::tr("ไม่มีรายการในเดือนนี้")} } }
                            div { class: "flow-entry-list", for item in &details.pending_items {
                                article { class: "flow-entry-detail", h3 { "{item.schedule.name().as_str()}" }
                                    strong { "{crate::i18n::currency_prefix()}{money_label(item.schedule.amount().money())}" }
                                    p { class: "muted", "{item.due} · {crate::i18n::tr(item.schedule.category().label())} · " {item.schedule.account().map(|id| account_label(&view, id)).unwrap_or_else(|| "—".into())} }
                                }
                            } }
                        } else {
                            { let entries: Vec<_> = if index == 0 { &details.expense_entries } else { &details.income_entries }.iter().filter(|e| category.is_none_or(|selected| matches!(e.kind(), ledger_domain::EntryKind::Expense { category, .. } if *category == selected)) && description.as_ref().is_none_or(|value| ledger_application::expense_description(e) == *value)).collect();
                            rsx! {
                                if category.is_some() {
                                    p { class: "flow-filter-total", {crate::i18n::tr("ยอดรวมรายการที่เลือก")}
                                        strong { {entries.iter().filter_map(|entry| entry_label(entry).map(|(_, amount, _)| amount)).try_fold(ledger_domain::Money::ZERO, |sum, amount| sum.checked_add(amount)).map(|amount| format!("{}{}", crate::i18n::currency_prefix(), money_label(amount))).unwrap_or_else(|_| "—".into())} }
                                    }
                                }
                                if entries.is_empty() { p { class: "muted", {crate::i18n::tr("ไม่มีรายการในเดือนนี้")} } }
                                div { class: "flow-entry-list", for entry in entries {
                                    if let Some((label, amount, account)) = entry_label(entry) {
                                        article { class: "flow-entry-detail", h3 { "{crate::i18n::tr(label)}" }
                                            strong { "{crate::i18n::currency_prefix()}{money_label(amount)}" }
                                            p { class: "muted", "{entry.date()} · {account_label(&view, account)}" }
                                            if !entry.note().as_str().is_empty() { p { class: "flow-entry-note", "{entry.note().as_str()}" } }
                                            crate::entry_breakdown::EntryBreakdown { view: view.clone(), entry: entry.clone() }
                                        }
                                    }
                                } }
                            } }
                        }
                    } }
                }
            }
        }
    }
}
