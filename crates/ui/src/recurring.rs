use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

fn empty_form(month: Month) -> RecurringInput {
    RecurringInput {
        installments: None,
        name: String::new(),
        amount: String::new(),
        day: "1".into(),
        start: month.to_string(),
        account: None,
        category: None,
    }
}

#[component]
pub(crate) fn RecurringPage(view: Dashboard) -> Element {
    let tab = use_signal(|| 0usize);
    let mut store = use_context::<UiState>();
    let current = match Month::of(view.today) {
        Ok(month) => month,
        Err(_) => return rsx! { p { {crate::i18n::text("วันที่ไม่ถูกต้อง", &[])} } },
    };
    let mut month = use_signal(|| current);
    let default_start = default_recurring_month(view.today, 1).unwrap_or(current);
    let mut show_form = use_signal(|| false);
    let mut filter = use_signal(|| 0_u8);
    let mut form = use_signal(|| empty_form(default_start));
    let mut paying = use_signal(|| None::<(RecurringExpense, Month)>);
    let mut stopping = use_signal(|| None::<RecurringExpense>);
    let mut configuring = use_signal(|| None::<RecurringExpense>);
    let mut editing = use_signal(|| None::<(RecurringExpense, Month)>);
    let mut inspecting = use_signal(|| None::<(RecurringExpense, Month)>);
    let mut known_count = use_signal(|| view.recurring.len());
    let mut known_settlements = use_signal(|| view.settlements.clone());
    use_effect(move || {
        if let Some(view) = store.view.read().as_ref() {
            if inspecting
                .peek()
                .as_ref()
                .is_some_and(|(s, _)| !view.recurring.contains(s))
            {
                inspecting.set(None);
            }
            if editing
                .peek()
                .as_ref()
                .is_some_and(|(s, _)| !view.recurring.contains(s))
            {
                editing.set(None);
            }
            if view.recurring.len() != *known_count.peek() {
                known_count.set(view.recurring.len());
                show_form.set(false);
                form.set(empty_form(default_start));
            }
            if view.settlements != *known_settlements.peek() {
                known_settlements.set(view.settlements.clone());
                paying.set(None);
            }
            if configuring
                .peek()
                .as_ref()
                .is_some_and(|s| !view.recurring.contains(s))
            {
                configuring.set(None);
            }
            if stopping
                .peek()
                .as_ref()
                .is_some_and(|s| !view.recurring.contains(s))
            {
                stopping.set(None);
            }
        }
    });
    let summary = match recurring_month(&view, month()) {
        Ok(summary) => summary,
        Err(error) => return rsx! { p { role: "alert", "{error}" } },
    };
    let progress = recurring_progress(&view);
    let recorded = summary
        .items
        .iter()
        .filter(|i| i.paid_entry.is_some())
        .count();
    let overdue = summary
        .items
        .iter()
        .filter(|i| i.paid_entry.is_none() && i.due < view.today)
        .count();
    let maximum = summary
        .planned
        .minor()
        .max(summary.paid.minor())
        .max(summary.pending.minor());
    let visible_items: Vec<_> = summary
        .items
        .iter()
        .filter(|item| match filter() {
            1 => item.paid_entry.is_none() && item.due < view.today,
            2 => item.paid_entry.is_none(),
            3 => item.paid_entry.is_some(),
            _ => true,
        })
        .collect();
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("หนี้และรายจ่ายประจำ", &[])} } p { class: "muted", {crate::i18n::text("วางแผนค่าเช่า ค่าน้ำไฟ ค่าสมาชิก และค่าใช้จ่ายที่ต้องจ่ายทุกเดือน", &[])} } }
            button { class: "primary", disabled: *store.busy.read(), onclick: move |_| show_form.set(!show_form()), {crate::i18n::text("เพิ่มรายจ่ายประจำ", &[])} }
        }
        PageTabs { id: "recurring", tabs: vec![("calendar", "บิลรายเดือน"), ("wallet", "บัตรเครดิต"), ("list", "แผนรายจ่าย"), ("up", "สัดส่วนบิล")], selected: tab }
        PagePanel { lazy: true, id: "recurring", index: 3, selected: tab(),
            section { class: "card recurring-card",
                div { class: "section-heading", h2 { {crate::i18n::tr("สัดส่วนหนี้ประจำเดือน")} }
                    input { r#type: "month", "aria-label": crate::i18n::tr("งวดเดือน (ค.ศ.)"), min: "1900-01", max: "9999-12", value: "{month}", onchange: move |e| { if let Ok(value) = e.value().parse() { month.set(value); } } }
                }
                p { class: "field-hint", {crate::i18n::tr("แสดงเฉพาะบิลที่ยังไม่บันทึกจ่ายในเดือนที่เลือก แตะชื่อเพื่อดูและแก้ไข")} }
                { let pending: Vec<_> = summary.items.iter().filter(|i| i.paid_entry.is_none()).map(|i| i.schedule.clone()).collect();
                  let slices = pending.iter().map(|s| crate::pie::PieSlice { label: s.name().as_str().into(), amount: s.amount().money() }).collect();
                  rsx! { crate::pie::PieChart { slices, label: crate::i18n::tr("บิลค้าง"), onselect: move |i: usize| { if let Some(s) = pending.get(i) { inspecting.set(Some((s.clone(), month()))); } } } }
                }
            }
        }
        PagePanel { lazy: true, id: "recurring", index: 1, selected: tab(), crate::debt_visuals::CreditCardsPanel { view: view.clone() } }
        PagePanel { lazy: true, id: "recurring", index: 0, selected: tab(),
        section { class: "card recurring-card monthly-obligations",
            div { class: "recurring-month",
                button { class: "text-button", "aria-label": crate::i18n::text("เดือนก่อน", &[]), onclick: move |_| { if let Ok(previous) = month().shifted(-1) { month.set(previous); } }, "←" }
                label { r#for: "recurring-month", {crate::i18n::text("งวดเดือน (ค.ศ.)", &[])} }
                input { id: "recurring-month", r#type: "month", min: "1900-01", max: "9999-12", value: "{month}", onchange: move |e| { if let Ok(value) = e.value().parse() { month.set(value); } } }
                button { class: "text-button", "aria-label": crate::i18n::text("เดือนถัดไป", &[]), onclick: move |_| { if let Ok(next) = month().shifted(1) { month.set(next); } }, "→" }
            }
            div { class: "debt-summary-visual",
                crate::debt_visuals::ProgressRing { done: recorded as i64, total: summary.items.len() as i64, label: crate::i18n::text("บันทึกแล้ว", &[]) }
                div { class: "debt-summary-bars",
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("ยอดตามแผน", &[]), amount: summary.planned, maximum, tone: "plan" }
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("ยอดที่บันทึกแล้ว", &[]), amount: summary.paid, maximum, tone: "paid" }
                    crate::debt_visuals::AmountBar { label: crate::i18n::text("ยังไม่บันทึกจ่าย", &[]), amount: summary.pending, maximum, tone: "due" }
                }
                div { class: "debt-summary-callout", Icon { name: "calendar", size: 28 }
                    strong { "{overdue}" } span { {crate::i18n::text("รายการเลยกำหนด", &[])} }
                    small { {crate::i18n::text("บันทึกแล้ว {0} จาก {1} รายการ", &[recorded.to_string(), summary.items.len().to_string()])} }
                }
            }
            p { class: "field-hint", {crate::i18n::text("แผนยังไม่หักเงิน · รายการที่ลงบัตรเครดิตแล้วจะไปอยู่ในยอดบัตรรอชำระ · เดือนสั้นใช้วันสุดท้ายของเดือน", &[])} }
            div { class: "debt-filters", "aria-label": crate::i18n::text("กรองสถานะรายจ่าย", &[]),
                for (value, label) in [(0, "ทั้งหมด"), (1, "เลยกำหนด"), (2, "ยังไม่จ่าย"), (3, "บันทึกแล้ว")] {
                    button { r#type: "button", "aria-pressed": filter() == value, onclick: move |_| filter.set(value), "{crate::i18n::tr(label)}" }
                }
            }
            if summary.items.is_empty() { p { class: "muted", {crate::i18n::text("ยังไม่มีรายจ่ายประจำในเดือนนี้ เพิ่มแผนเพื่อดูวันครบกำหนดและยอดรวม", &[])} } }
            else if visible_items.is_empty() { p { class: "muted", {crate::i18n::text("ไม่มีรายการในสถานะนี้", &[])} } }
            for item in visible_items {
                { let schedule = item.schedule.clone(); let for_details = schedule.clone(); let for_edit = schedule.clone(); let for_pay = schedule.clone(); let period = month(); let unpaid = item.paid_entry.is_none(); let edit_icon = if unpaid { "edit" } else { "list" };
                  rsx! {
                    article { key: "{schedule.id()}-{period}", class: "recurring-item obligation-row", "data-overdue": item.paid_entry.is_none() && item.due < view.today,
                        button { r#type: "button", class: "obligation-preview", "aria-haspopup": "dialog", "aria-label": crate::i18n::text("ดูรายละเอียด {0}", &[schedule.name().as_str().into()]), onclick: move |_| inspecting.set(Some((for_details.clone(), period))),
                            span { class: "due-date-tile", strong { {item.due.date().format("%d").to_string()} } small { {item.due.date().format("%m / %Y").to_string()} } }
                            span { class: "obligation-copy",
                                strong { class: "obligation-title", "{schedule.name().as_str()}" }
                                span { {crate::i18n::text("ครบกำหนด {0} · ทุกวันที่ {1}", &[format!("{}", item.due), format!("{}", schedule.due().day())])} }
                                span { class: "field-hint", {crate::i18n::text("{0} · {1}", &[crate::i18n::tr(schedule.category().label()).to_string(), schedule.account().map(|id| account_label(&view, id)).unwrap_or_else(|| crate::i18n::tr("กรุณาเลือกบัญชีใหม่เมื่อบันทึกจ่าย"))])} }
                                span { class: "obligation-details-link", {crate::i18n::tr("ดูรายละเอียด")} Icon { name: "arrow-right", size: 16 } }
                            }
                        }
                        div { class: "recurring-item-actions", strong { "{crate::i18n::currency_prefix()}{money_label(schedule.amount().money())}" }
                            span { class: "obligation-status", "data-overdue": item.paid_entry.is_none() && item.due < view.today, "{occurrence_status(item, &view)}" }
                            div { class: "obligation-buttons",
                                button { class: "soft-button", disabled: *store.busy.read(), "aria-haspopup": "dialog", onclick: move |_| {
                                    store.notice.set(None);
                                    if for_edit.occurs_in(period) && unpaid { editing.set(Some((for_edit.clone(), period))); }
                                    else { inspecting.set(Some((for_edit.clone(), period))); }
                                }, Icon { name: edit_icon, size: 18 } {crate::i18n::tr(if unpaid { "แก้ไขแผน" } else { "ดูรายละเอียด" })} }
                                if item.paid_entry.is_none() {
                                    button { class: "primary", disabled: *store.busy.read(), onclick: move |_| { store.recurring_prepared.set(None); paying.set(Some((for_pay.clone(), period))); }, Icon { name: "check", size: 18 } {crate::i18n::text("บันทึกจ่าย", &[])} }
                                }
                            }
                        }
                    }
                  }
                }
            }
        }
        }
        PagePanel { lazy: true, id: "recurring", index: 2, selected: tab(),
        section { class: "card recurring-card installment-plans",
            h2 { {crate::i18n::text("แผนที่ยังจ่ายไม่ครบ", &[])} }
            p { class: "field-hint", {crate::i18n::text("งวดค้างยังอยู่แม้ผ่านเดือนสุดท้ายแล้ว เลือกบันทึกงวดค้างหรือจ่ายล่วงหน้าได้", &[])} }
            if progress.iter().all(|p| p.next_unpaid.is_none()) { p { {crate::i18n::text("ไม่มีแผนที่ต้องจ่ายต่อแล้ว", &[])} } }
            for p in progress.iter().filter(|p| p.next_unpaid.is_some()) {
                { let schedule = p.schedule.clone(); let config = schedule.clone(); let for_edit = schedule.clone();
                  rsx! { article { class: "recurring-item installment-plan",
                    div { h3 { "{schedule.name().as_str()}" }
                        if let Some(count) = schedule.due().installments() {
                            progress { class: "installment-progress", max: "{count}", value: "{p.paid}", "aria-label": crate::i18n::text("ความคืบหน้าการชำระ", &[]) }
                        } else { div { class: "open-ended-plan", Icon { name: "calendar", size: 18 } {crate::i18n::text("ต่อเนื่องทุกเดือน", &[])} } }
                        if let Some(remaining) = p.remaining { p { {crate::i18n::text("จ่ายแล้ว {0}/{1} งวด · เหลือ {2} งวด", &[format!("{}", p.paid), format!("{}", schedule.due().installments().unwrap_or(0)), format!("{}", remaining)])} } }
                        else { p { {crate::i18n::text("ไม่กำหนดจำนวนงวด · จ่ายแล้ว {0} งวด", &[format!("{}", p.paid)])} } }
                    }
                    div { class: "recurring-item-actions",
                        if let Some(next) = p.next_unpaid {
                            p { "{crate::recurring_picker::installment_label(&schedule, next)}" }
                            button { class: "primary", disabled: *store.busy.read(), onclick: move |_| { store.recurring_prepared.set(None); paying.set(Some((schedule.clone(), next))); }, {crate::i18n::text("บันทึกงวดที่ยังไม่จ่าย", &[])} }
                        }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| configuring.set(Some(config.clone())), {crate::i18n::text("ตั้งจำนวนงวด", &[])} }
                        if let Some(next) = p.next_unpaid {
                            button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.notice.set(None); editing.set(Some((for_edit.clone(), next))); }, Icon { name: "edit", size: 18 } {crate::i18n::tr("แก้ไขแผน")} }
                        }
                    }
                  } }
                }
            }
            details { class: "flow-explanation", summary { {crate::i18n::text("แผนที่จ่ายครบ / หยุดแล้ว", &[])} }
                for p in progress.iter().filter(|p| p.next_unpaid.is_none()) {
                    { let schedule = p.schedule.clone(); rsx! { article { class: "recurring-item",
                        div { strong { "{schedule.name().as_str()}" }
                            if p.completed { p { class: "status-pill", {crate::i18n::text("จ่ายครบ {0} งวดแล้ว", &[format!("{}", p.paid)])} } }
                            else { p { {crate::i18n::text("หยุดแผนแล้ว · จ่ายแล้ว {0} งวด", &[format!("{}", p.paid)])} } }
                        }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| configuring.set(Some(schedule.clone())), {crate::i18n::text("ตั้งจำนวนงวด", &[])} }
                    } } }
                }
            }
            p { class: "field-hint", {crate::i18n::text("กดแก้ไขแผนเพื่อเปลี่ยนยอด วันชำระ หรือบัญชีตั้งแต่งวดที่เลือก · ชำระหนี้บัตรเครดิตด้วยการโอนเข้าบัญชีบัตร ไม่ลงรายจ่ายซ้ำ", &[])} }
        }
        }
        if show_form() {
            dialog { id: "rec-create-dialog", class: "account-dialog recurring-dialog", "aria-labelledby": "rec-create-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('rec-create-dialog').showModal(); document.getElementById('new-rec-name').focus()"); },
                oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { show_form.set(false); } },
                div { class: "section-heading", h2 { id: "rec-create-title", {crate::i18n::text("เพิ่มแผนรายจ่ายประจำ", &[])} }
                    button { class: "icon-button", "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), disabled: *store.busy.read(), onclick: move |_| show_form.set(false), Icon { name: "close", size: 20 } }
                }
                if let Some((true, error)) = (store.notice)() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&error)}" } }
                form { onsubmit: move |event| { event.prevent_default(); store.send(Command::AddRecurring(form())); },
                    RecurringFields { form, view: view.clone(), prefix: "new", baseline: None }
                    p { class: "field-hint", {crate::i18n::text("ยอดนี้เป็นแผนรายเดือน ก่อนจ่ายจริงสามารถแก้ยอดและบัญชีได้", &[])} }
                    button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("บันทึกแผนรายเดือน", &[])} }
                    button { class: "text-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| show_form.set(false), {crate::i18n::text("ยกเลิก", &[])} }
                }
            }
        }
        if let Some(schedule) = configuring() {
            InstallmentsDialog { key: "{schedule.id()}", schedule, onclose: move |_| configuring.set(None) }
        }
        if let Some((schedule, period)) = inspecting() {
            RecurringDetailsDialog { key: "{schedule.id()}-{period}", schedule: schedule.clone(), period, view: view.clone(),
                onclose: move |_| inspecting.set(None),
                onedit: { let schedule = schedule.clone(); move |_| { inspecting.set(None); store.notice.set(None); editing.set(Some((schedule.clone(), period))); } },
                onpay: { let schedule = schedule.clone(); move |_| { inspecting.set(None); store.recurring_prepared.set(None); paying.set(Some((schedule.clone(), period))); } },
                onconfigure: { let schedule = schedule.clone(); move |_| { inspecting.set(None); configuring.set(Some(schedule.clone())); } },
                onstop: move |_| { inspecting.set(None); month.set(period); stopping.set(Some(schedule.clone())); }
            }
        }
        if let Some((schedule, effective)) = editing() {
            EditRecurringDialog { key: "{schedule.id()}-{effective}", schedule, effective, view: view.clone(), onclose: move |_| editing.set(None) }
        }
        if let Some((schedule, period)) = paying() {
            PaymentDialog { key: "{schedule.id()}-{period}", schedule, month: period, view: view.clone(), onclose: move |_| { paying.set(None); store.recurring_prepared.set(None); } }
        }
        if let Some(schedule) = stopping() {
            dialog { id: "rec-stop-dialog", class: "account-dialog recurring-dialog", "aria-label": crate::i18n::text("ยืนยันหยุดรายจ่ายประจำ", &[]),
                onmounted: move |_| { let _ = document::eval("document.getElementById('rec-stop-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { stopping.set(None); } },
                if let Some((true, error)) = store.notice.read().clone() { p { role: "alert", class: "form-error", "{error}" } }
                h2 { {crate::i18n::text("หยุด {0} ตั้งแต่ {1}", &[schedule.name().as_str().to_string(), format!("{}", month)])} }
                p { {crate::i18n::text("ยอดที่ยังไม่จ่ายตั้งแต่งวดนี้จะไม่รวมในแผนและ Flow rate อีก รายจ่ายที่บันทึกแล้วจะยังอยู่ หากเป็นเพียงการจ่ายแล้ว ให้ใช้บันทึกการจ่ายแทน", &[])} }
                button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::StopRecurring { expected: schedule.clone(), month: month() }), {crate::i18n::text("ยืนยันหยุดแผน", &[])} }
                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| stopping.set(None), {crate::i18n::text("กลับ", &[])} }
            }
        }
    }
}

fn occurrence_status(item: &RecurringOccurrence, view: &Dashboard) -> String {
    if item.paid_entry.is_some() {
        if view.entries.iter().any(|e| Some(e.id()) == item.paid_entry && matches!(e.kind(), EntryKind::Expense { account, .. } if view.accounts.iter().any(|a| a.account.id() == *account && a.account.kind() == AccountKind::CreditCard))) {
            crate::i18n::tr("ลงบัตรแล้ว · รอชำระบัตร")
        } else {
            format!("{} {}{}", crate::i18n::tr("จ่ายแล้ว"), crate::i18n::currency_prefix(), money_label(item.paid_amount))
        }
    } else {
        crate::i18n::tr(if item.due < view.today {
            "เลยกำหนด · ยังไม่บันทึกจ่าย"
        } else {
            "ยังไม่จ่าย"
        })
    }
}

#[component]
fn RecurringDetailsDialog(
    schedule: RecurringExpense,
    period: Month,
    view: Dashboard,
    onclose: EventHandler,
    onedit: EventHandler,
    onpay: EventHandler,
    onconfigure: EventHandler,
    onstop: EventHandler,
) -> Element {
    let store = use_context::<UiState>();
    let item = recurring_month(&view, period).ok().and_then(|m| {
        m.items
            .into_iter()
            .find(|i| i.schedule.id() == schedule.id())
    });
    let unpaid = item.as_ref().is_some_and(|i| i.paid_entry.is_none());
    rsx! {
        dialog { id: "rec-details-dialog", class: "account-dialog recurring-dialog recurring-details-dialog", "aria-labelledby": "rec-details-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('rec-details-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); onclose.call(()); },
            div { class: "section-heading",
                h2 { id: "rec-details-title", "{schedule.name().as_str()}" }
                button { class: "icon-button", "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| onclose.call(()), Icon { name: "close", size: 20 } }
            }
            p { class: "recurring-detail-amount", "{crate::i18n::currency_prefix()}{money_label(schedule.amount().money())}" }
            if let Some(item) = &item { p { class: "obligation-status", "data-overdue": unpaid && item.due < view.today, "{occurrence_status(item, &view)}" } }
            dl { class: "recurring-detail-fields",
                div { dt { {crate::i18n::tr("วันครบกำหนด")} } dd { if let Some(item) = &item { "{item.due}" } } }
                div { dt { {crate::i18n::tr("งวดชำระ")} } dd { "{crate::recurring_picker::installment_label(&schedule, period)}" } }
                div { dt { {crate::i18n::tr("ทุกวันที่ของเดือน (1–31)")} } dd { "{schedule.due().day()}" } }
                div { dt { {crate::i18n::tr("บัญชีที่จะใช้จ่าย")} } dd { {schedule.account().map(|id| account_label(&view, id)).unwrap_or_else(|| crate::i18n::tr("กรุณาเลือกบัญชีใหม่เมื่อบันทึกจ่าย"))} } }
                div { dt { {crate::i18n::tr("หมวดรายจ่าย")} } dd { {crate::i18n::tr(schedule.category().label())} } }
            }
            if let Some(end) = schedule.stopped_from() { p { class: "field-hint", {crate::i18n::text("แผนนี้หยุดตั้งแต่ {0} · แก้ไขได้เฉพาะงวดก่อนเดือนนี้ โดยไม่เปิดแผนกลับมา", &[end.to_string()])} } }
            if !unpaid { p { class: "field-hint", {crate::i18n::tr("งวดนี้มีประวัติจ่ายแล้ว เก็บรายละเอียดเดิมไว้ หากต้องการเปลี่ยนแผนให้เลือกงวดที่ยังไม่จ่าย")} } }
            div { class: "recurring-detail-actions",
                if unpaid {
                    button { class: "primary", disabled: (store.busy)(), onclick: move |_| onpay.call(()), Icon { name: "check", size: 18 } {crate::i18n::tr("บันทึกจ่าย")} }
                    button { class: "soft-button", disabled: (store.busy)(), onclick: move |_| onedit.call(()), Icon { name: "edit", size: 18 } {crate::i18n::tr("แก้ไขแผน")} }
                }
                button { class: "text-button", disabled: (store.busy)(), onclick: move |_| onconfigure.call(()), {crate::i18n::tr("ตั้งจำนวนงวด")} }
                if schedule.stopped_from().is_none() {
                    button { class: "text-button", disabled: (store.busy)(), onclick: move |_| onstop.call(()), {crate::i18n::tr("หยุดแผนตั้งแต่งวดนี้")} }
                }
            }
        }
    }
}

#[component]
fn PaymentDialog(
    schedule: RecurringExpense,
    month: Month,
    view: Dashboard,
    onclose: EventHandler,
) -> Element {
    let mut store = use_context::<UiState>();
    let mut input = use_signal(|| recurring_payment_input(&schedule, view.today));
    let mut existing = use_signal(String::new);
    let prepared = store.recurring_prepared.read().clone();
    let candidates: Vec<_> = view
        .entries
        .iter()
        .filter(|e| {
            matches!(e.kind(), EntryKind::Expense { .. })
                && !view.reversed.contains(&e.id())
                && !view.settlements.iter().any(|s| s.entry == e.id())
        })
        .collect();
    rsx! {
        dialog { id: "rec-pay-dialog", class: "account-dialog recurring-dialog", "aria-label": crate::i18n::text("บันทึกจ่ายรายจ่ายประจำ", &[]),
            onmounted: move |_| { let _ = document::eval("document.getElementById('rec-pay-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { onclose.call(()); } },
            if let Some((true, error)) = store.notice.read().clone() { p { role: "alert", class: "form-error", "{error}" } }
            h2 { "{schedule.name().as_str()}" }
            p { "{crate::recurring_picker::installment_label(&schedule, month)}" }
            if let Some(prepared) = prepared {
                p { {crate::i18n::text("ตรวจยอดจ่ายจริง ฿{0}", &[money_label(entry_label(&prepared.payment.entry).map(|(_, amount, _)| amount).unwrap_or(Money::ZERO)).to_string()])} }
                p { {crate::i18n::text("วันที่ {0} · {1}", &[format!("{}", prepared.payment.entry.date()), input.read().account.map(|id| account_label(&view, id)).unwrap_or_default().to_string()])} }
                p { {crate::i18n::text("จะลงรายจ่ายหนึ่งรายการ และเปลี่ยนงวดนี้เป็นจ่ายแล้ว ยอดแผนจะไม่ถูกบวกซ้ำใน Flow rate", &[])} }
                button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::PayRecurring(prepared.clone())), {crate::i18n::text("ยืนยันจ่ายและบันทึกรายจ่าย", &[])} }
                button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.recurring_prepared.set(None), {crate::i18n::text("กลับไปแก้", &[])} }
            } else {
                label { r#for: "rec-existing", {crate::i18n::text("เคยบันทึกรายจ่ายนี้แล้วหรือยัง", &[])} }
                select { id: "rec-existing", disabled: *store.busy.read(), value: "{existing}", onchange: move |e| existing.set(e.value()),
                    option { value: "", selected: existing().is_empty(), {crate::i18n::text("ยัง — บันทึกรายจ่ายใหม่", &[])} }
                    for entry in candidates { if let Some((_, amount, account)) = entry_label(entry) {
                        option { value: "{entry.id()}", selected: existing() == entry.id().to_string(), "{entry.date()} · {entry.note().as_str()} · {account_label(&view, account)} · {crate::i18n::currency_prefix()}{money_label(amount)}" }
                    } }
                }
                if !existing().is_empty() {
                    p { class: "batch-question", {crate::i18n::text("จะผูกงวดนี้กับรายจ่ายที่เลือก โดยไม่สร้างรายจ่ายเพิ่ม ตรวจยอด วันที่ และบัญชีให้ตรงก่อนยืนยัน", &[])} }
                    button { class: "primary", disabled: *store.busy.read(), onclick: move |_| { if let Ok(entry) = existing().parse() { store.send(Command::LinkRecurring { expected: schedule.clone(), month, entry }); } }, {crate::i18n::text("ยืนยันใช้รายการเดิม", &[])} }
                } else {
                    form { onsubmit: move |e| { e.prevent_default(); store.send(Command::PreviewRecurringPayment { expected: schedule.clone(), month, input: input() }); },
                        fieldset { class: "recurring-fields", disabled: *store.busy.read(),
                            div { label { r#for: "rec-pay-amount", {crate::i18n::text("ยอดจ่ายจริง (บาท)", &[])} } input { id: "rec-pay-amount", required: true, inputmode: "decimal", value: input.read().amount.clone(), oninput: move |e| input.write().amount = e.value() } }
                            div { label { r#for: "rec-pay-date", {crate::i18n::text("วันที่จ่ายจริง", &[])} } input { id: "rec-pay-date", r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: input.read().date.clone(), onchange: move |e| input.write().date = e.value() } }
                            div { label { r#for: "rec-pay-account", {crate::i18n::text("บัญชีที่จ่าย", &[])} }
                                select { id: "rec-pay-account", required: true, value: input.read().account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| input.write().account = e.value().parse().ok(),
                                    option { value: "", selected: input.read().account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                                    for a in &view.accounts { if a.account.accepts_cash_entries() { option { value: "{a.account.id()}", selected: input.read().account == Some(a.account.id()), "{a.account.name().as_str()}" } } }
                                }
                            }
                        }
                        p { class: "field-hint", {crate::i18n::text("จ่ายล่วงหน้าได้ โดยระบุวันที่จ่ายจริง · หากเคยจดในบันทึกด่วน ให้เลือกใช้รายการเดิมด้านบน", &[])} }
                        button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("ตรวจยอดจ่ายก่อนบันทึก", &[])} }
                    }
                }
            }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| onclose.call(()), {crate::i18n::text("ปิด", &[])} }
        }
    }
}

#[component]
fn InstallmentsDialog(schedule: RecurringExpense, onclose: EventHandler) -> Element {
    let store = use_context::<UiState>();
    let mut count = use_signal(|| schedule.due().installments().map(|n| n.to_string()));
    rsx! {
        dialog { id: "rec-count-dialog", class: "account-dialog", "aria-label": crate::i18n::text("ตั้งจำนวนงวด", &[]),
            onmounted: move |_| { let _ = document::eval("document.getElementById('rec-count-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !*store.busy.read() { onclose.call(()); } },
            h2 { {crate::i18n::text("ตั้งจำนวนงวด · {0}", &[schedule.name().as_str().to_string()])} }
            if let Some((true, error)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{error}" } }
            form { onsubmit: move |e| {
                e.prevent_default();
                if parse_installments(count().as_deref()).is_ok_and(|value| value == schedule.due().installments()) {
                    onclose.call(());
                } else {
                    store.send(Command::SetRecurringInstallments { expected: schedule.clone(), installments: count() });
                }
            },
                label { class: "flow-mode", input { r#type: "checkbox", checked: count.read().is_some(), disabled: *store.busy.read(), onchange: move |e| count.set(e.checked().then(String::new)) } {crate::i18n::text("กำหนดจำนวนงวด", &[])} }
                if let Some(value) = count() {
                    label { r#for: "edit-installments", {crate::i18n::text("จำนวนงวดทั้งหมด รวมงวดที่จ่ายแล้ว", &[])} }
                    input { id: "edit-installments", r#type: "number", min: "1", max: "1200", required: true, value, disabled: *store.busy.read(), oninput: move |e| count.set(Some(e.value())) }
                } else { p { {crate::i18n::text("ไม่กำหนดจำนวนงวด — เกิดซ้ำทุกเดือนจนกว่าจะหยุดแผน", &[])} } }
                p { class: "field-hint", {crate::i18n::text("จำนวนงวดนับจากเดือนเริ่มต้นของแผน จ่ายครบแล้วจะไม่มีงวดถัดไป ประวัติที่เคยจ่ายจะไม่ถูกตัดทิ้ง", &[])} }
                button { class: "primary", r#type: "submit", disabled: *store.busy.read(), {crate::i18n::text("บันทึกจำนวนงวด", &[])} }
                button { class: "text-button", r#type: "button", disabled: *store.busy.read(), onclick: move |_| onclose.call(()), {crate::i18n::text("กลับ", &[])} }
            }
        }
    }
}

#[component]
fn RecurringFields(
    mut form: Signal<RecurringInput>,
    view: Dashboard,
    prefix: &'static str,
    baseline: Option<RecurringExpense>,
) -> Element {
    let store = use_context::<UiState>();
    let mut manual_start = use_signal(|| baseline.is_some());
    let is_edit = baseline.is_some();
    let today = view.today;
    rsx! {
                    fieldset { disabled: *store.busy.read(), class: "recurring-fields",
                        div { label { r#for: "{prefix}-rec-name", {crate::i18n::text("ชื่อค่าใช้จ่าย", &[])} } input { id: "{prefix}-rec-name", required: true, maxlength: 60, placeholder: crate::i18n::text("เช่น ค่าเช่าห้อง", &[]), value: form.read().name.clone(), oninput: move |e| form.write().name = e.value() } }
                        div { label { r#for: "{prefix}-rec-amount", {crate::i18n::text("ยอดต่อเดือน (บาท)", &[])} } input { id: "{prefix}-rec-amount", required: true, inputmode: "decimal", placeholder: "8500.00", value: form.read().amount.clone(), oninput: move |e| form.write().amount = e.value() } }
                        div { label { r#for: "{prefix}-rec-day", {crate::i18n::text("ทุกวันที่ของเดือน (1–31)", &[])} } input { id: "{prefix}-rec-day", r#type: "number", min: "1", max: "31", required: true, value: form.read().day.clone(), oninput: move |e| {
                            let day = e.value();
                            if !manual_start() && !is_edit && let Ok(day) = day.parse() && let Ok(month) = default_recurring_month(today, day) {
                                form.write().start = month.to_string();
                            }
                            form.write().day = day;
                        } } }
                        div { label { r#for: "{prefix}-rec-start", {crate::i18n::tr(if baseline.is_some() { "เริ่มใช้การแก้ไขตั้งแต่งวด (ค.ศ.)" } else { "เริ่มงวดเดือน (ค.ศ.)" })} } input { id: "{prefix}-rec-start", r#type: "month", min: "1900-01", max: "9999-12", required: true, value: form.read().start.clone(), onchange: move |e| {
                let start = e.value();
                manual_start.set(true);
                    if let (Some(schedule), Ok(month)) = (&baseline, start.parse::<Month>()) {
                        form.write().installments = recurring_edit_input(schedule, month).installments;
                    }
                    form.write().start = start;
                } } }
                        div { class: "installments-field",
                            label { class: "flow-mode", input { r#type: "checkbox", checked: form.read().installments.is_some(), onchange: move |e| form.write().installments = e.checked().then(String::new) } {crate::i18n::text("กำหนดจำนวนงวด", &[])} }
                            if let Some(count) = form.read().installments.clone() {
                                label { r#for: "{prefix}-rec-count", {crate::i18n::text("จำนวนงวดทั้งหมด (นับจากเดือนเริ่ม)", &[])} }
                                input { id: "{prefix}-rec-count", r#type: "number", min: "1", max: "1200", required: true, placeholder: crate::i18n::text("เช่น 12", &[]), value: count, oninput: move |e| form.write().installments = Some(e.value()) }
                            } else { p { class: "field-hint", {crate::i18n::text("ไม่กำหนดจำนวนงวด — เกิดซ้ำทุกเดือนจนกว่าจะหยุดแผน", &[])} } }
                        }
                        div { label { r#for: "{prefix}-rec-account", {crate::i18n::text("บัญชีที่จะใช้จ่าย", &[])} }
                            select { id: "{prefix}-rec-account", required: true, value: form.read().account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |e| form.write().account = e.value().parse().ok(),
                                option { value: "", selected: form.read().account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                                for a in &view.accounts { if a.account.accepts_cash_entries() { option { value: "{a.account.id()}", selected: form.read().account == Some(a.account.id()), "{a.account.name().as_str()}" } } }
                            }
                        }
                        div { label { r#for: "{prefix}-rec-category", {crate::i18n::text("หมวดรายจ่าย", &[])} }
                            select { id: "{prefix}-rec-category", required: true, value: form.read().category.map(|c| c.code()).unwrap_or(""), onchange: move |e| form.write().category = Category::from_code(&e.value()).ok(),
                                option { value: "", selected: form.read().category.is_none(), {crate::i18n::text("เลือกหมวดหมู่", &[])} }
                                for c in Category::EXPENSE { option { value: c.code(), selected: form.read().category == Some(c), "{crate::i18n::tr(c.label())}" } }
                            }
                        }
                    }
                    if !is_edit { p { class: "field-hint", {crate::i18n::tr("วันครบกำหนดที่ผ่านไปแล้วจะเริ่มเดือนถัดไปโดยอัตโนมัติ เลือกเดือนเองได้หากต้องการบันทึกย้อนหลัง")} } }
    }
}

#[component]
fn EditRecurringDialog(
    schedule: RecurringExpense,
    effective: Month,
    view: Dashboard,
    onclose: EventHandler,
) -> Element {
    let store = use_context::<UiState>();
    let form = use_signal(|| recurring_edit_input(&schedule, effective));
    let expected = schedule.clone();
    rsx! {
        dialog { id: "rec-edit-dialog", class: "account-dialog recurring-dialog", "aria-labelledby": "rec-edit-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('rec-edit-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !(store.busy)() { onclose.call(()); } },
            div { class: "section-heading", h2 { id: "rec-edit-title", {crate::i18n::tr("แก้ไขแผน")} }
                button { class: "icon-button", disabled: (store.busy)(), "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| onclose.call(()), Icon { name: "close", size: 20 } }
            }
            p { class: "field-hint", {crate::i18n::tr("การแก้ไขมีผลตั้งแต่งวดที่เลือก งวดก่อนหน้านั้นและรายการที่จ่ายแล้วเก็บข้อมูลเดิม หากจ่ายล่วงหน้าแล้วให้เลือกงวดหลังรายการที่จ่ายล่าสุด")} }
            if let Some(end) = schedule.stopped_from() { p { class: "field-hint", {crate::i18n::text("แผนนี้หยุดตั้งแต่ {0} · แก้ไขได้เฉพาะงวดก่อนเดือนนี้ โดยไม่เปิดแผนกลับมา", &[end.to_string()])} } }
            form { onsubmit: move |e| {
                e.prevent_default();
                if form() == recurring_edit_input(&expected, effective) { onclose.call(()); }
                else { store.send(Command::EditRecurring { expected: expected.clone(), input: form() }); }
            },
                RecurringFields { form, view, prefix: "edit", baseline: Some(schedule) }
                if let Some((true, error)) = (store.notice)() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&error)}" } }
                div { class: "crypto-account-actions",
                    button { class: "primary", r#type: "submit", disabled: (store.busy)(), {crate::i18n::tr("บันทึกการแก้ไข")} }
                    button { class: "soft-button", r#type: "button", disabled: (store.busy)(), onclick: move |_| onclose.call(()), {crate::i18n::tr("ยกเลิก")} }
                }
            }
        }
    }
}
