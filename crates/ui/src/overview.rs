use crate::{HostInfo, components::*, pages::TransactionRows, state::*};
use chrono::Datelike;
use dioxus::prelude::*;
use ledger_application::Dashboard;

fn needs_annual_backup(
    today: chrono::NaiveDate,
    dismissed_year: Option<i32>,
    entry_years: impl Iterator<Item = i32>,
) -> bool {
    today.month() == 1
        && dismissed_year != Some(today.year())
        && entry_years.into_iter().any(|year| year < today.year())
}

#[cfg(test)]
mod annual_backup_tests {
    use super::needs_annual_backup;
    use chrono::NaiveDate;

    #[test]
    fn reminder_requires_new_year_history_and_respects_this_year_dismissal()
    -> Result<(), &'static str> {
        let january = NaiveDate::from_ymd_opt(2027, 1, 1).ok_or("test date")?;
        assert!(needs_annual_backup(january, None, [2026, 2027].into_iter()));
        assert!(!needs_annual_backup(january, None, [2027].into_iter()));
        assert!(!needs_annual_backup(january, None, [].into_iter()));
        assert!(!needs_annual_backup(
            january,
            Some(2027),
            [2026].into_iter()
        ));
        assert!(needs_annual_backup(january, Some(2026), [2026].into_iter()));
        for (month, day) in [(2, 1), (12, 31)] {
            let date = NaiveDate::from_ymd_opt(2027, month, day).ok_or("test date")?;
            assert!(!needs_annual_backup(date, None, [2026].into_iter()));
        }
        Ok(())
    }
}

#[component]
pub fn Overview(view: Dashboard) -> Element {
    let tab = use_signal(|| 0usize);
    let market = use_context::<crate::crypto::CryptoMarket>();
    let mut view = view;
    let valuation = ledger_application::crypto_valuation(&view, (market.prices)(), (market.now)());
    let has_crypto = view
        .accounts
        .iter()
        .any(|a| a.account.crypto_holdings().is_some());
    let incomplete = valuation.as_ref().is_ok_and(|v| v.unpriced_portfolios > 0);
    let invalid_value = valuation.is_err();
    if let Ok(value) = valuation {
        view.assets = value.assets;
        view.liabilities = value.liabilities;
        view.net_worth = value.net_worth;
    }
    let mut store = use_context::<UiState>();
    let mut backup_reminder_dismissed = store.backup_reminder_dismissed;
    let host = use_context::<HostInfo>();
    let date = view.today.date();
    let month = thai_month(date.month());
    let year = crate::i18n::year(date.year());
    let cashflow = view
        .income
        .checked_sub(view.expenses)
        .map(money_label)
        .unwrap_or_else(|_| "—".into());
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("ภาพรวม", &[])} } } span { class: "date-pill", "{date.day()} {month} {year}" } }
        if needs_annual_backup(date, backup_reminder_dismissed(), view.entries.iter().map(|e| e.date().month_key().0)) {
            aside { class: "annual-backup-reminder", role: "status",
                div { strong { {crate::i18n::tr("สำรองข้อมูลรับปีใหม่")} } p { {crate::i18n::tr("เก็บประวัติปีก่อนไว้ครบ แนะนำให้สำรองทั้งแอปก่อนเริ่มปีใหม่")} } }
                button { class: "primary", onclick: move |_| { store.settings_tab.set(3); store.page.set(Page::Settings); }, {crate::i18n::tr("ไปสำรองข้อมูล")} }
                button { class: "text-button", onclick: move |_| backup_reminder_dismissed.set(Some(date.year())), {crate::i18n::tr("ไว้ภายหลัง")} }
            }
        }
        PageTabs { id: "overview", tabs: vec![("home", "สรุปบัญชี"), ("up", "รายจ่าย"), ("list", "รายการล่าสุด"), ("wallet", "ฐานะการเงิน"), ("up", "กระแสเงินสด"), ("file", "หนี้บัตรเครดิต"), ("user", "ลูกหนี้")], selected: tab }
        PagePanel { lazy: true, id: "overview", index: 0, selected: tab(),
        if incomplete { p { class: "notice error", role: "status", {crate::i18n::tr("ยอดภาพรวมยังไม่ครบ: ยังไม่รวมพอร์ตที่ไม่มีราคา กรุณาอัปเดตราคาตลาด")} } }
        if invalid_value { p { class: "notice error", role: "alert", {crate::i18n::tr("มูลค่าเกินขอบเขตที่คำนวณได้")} } }
        div { class: "overview-top",
            section { class: "hero card",
                div { class: "hero-copy", h2 { {crate::i18n::text("สินทรัพย์สุทธิ", &[])} }
                    div { class: "hero-amount", span { class: "currency", "{crate::i18n::currency().code()}" } if invalid_value { "—" } else { "{money_label(view.net_worth)}" } }
                    p { {crate::i18n::text("รวม {0} บัญชี · ข้อมูลในเครื่อง", &[format!("{}", view.accounts.len())])} }
                    if view.accounts.is_empty() {
                        button { class: "primary", onclick: move |_| store.account_form.set(true), Icon { name: "plus", size: 17 } {crate::i18n::text("เพิ่มบัญชีแรก", &[])} }
                    } else { button { class: "soft-button", onclick: move |_| store.page.set(Page::Accounts), {crate::i18n::text("ดูบัญชีของเรา", &[])} Icon { name: "arrow", size: 16 } } }
                }
                div { class: "hero-art", "aria-hidden": "true", img { src: host.art.hero.clone(), alt: "", draggable: false } }
                if !invalid_value { div { class: "hero-details",
                    div { span { class: "detail-icon", Icon { name: "wallet", size: 19 } } div { small { {crate::i18n::text("สินทรัพย์", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(view.assets)}" } } }
                    div { span { class: "detail-icon", Icon { name: "file", size: 19 } } div { small { {crate::i18n::text("หนี้สิน", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(view.liabilities)}" } } }
                } }
            }
            div { class: "month-cards",
                section { class: "card metric income", div { class: "metric-title", span { {crate::i18n::text("รายรับเดือนนี้", &[])} } span { class: "metric-icon", Icon { name: "down", size: 19 } } } strong { "{crate::i18n::currency_prefix()}{money_label(view.income)}" } small { "{month} {year}" } }
                section { class: "card metric expense", div { class: "metric-title", span { {crate::i18n::text("รายจ่ายเดือนนี้", &[])} } span { class: "metric-icon", Icon { name: "up", size: 19 } } } strong { "{crate::i18n::currency_prefix()}{money_label(view.expenses)}" } small { {crate::i18n::text("ไม่รวมการโอนระหว่างบัญชี", &[])} } }
                div { class: "cashflow", span { {crate::i18n::text("รายรับ − รายจ่ายที่บันทึก", &[])} } strong { "{crate::i18n::currency_prefix()}{cashflow}" } }
            }
        }
        if has_crypto { div { class: "bottom-market", crate::crypto::CryptoMarketStatus {} } }
        }
        PagePanel { lazy: true, id: "overview", index: 1, selected: tab(),
            section { class: "card spending", div { class: "section-heading", h2 { {crate::i18n::text("เงินไปไหนบ้าง", &[])} } span { class: "muted small", {crate::i18n::text("เดือนนี้", &[])} } } ExpenseChart { view: view.clone() } }
        }
        PagePanel { lazy: true, id: "overview", index: 2, selected: tab(),
            section { class: "card recent", div { class: "section-heading", h2 { {crate::i18n::text("เรื่องเงินล่าสุด", &[])} } button { class: "text-button", onclick: move |_| store.page.set(Page::Transactions), {crate::i18n::text("ดูทั้งหมด", &[])} Icon { name: "arrow", size: 14 } } }
                TransactionRows { view: view.clone(), limit: 4, allow_cancel: false }
                if view.thai_tax_enabled && view.currency == ledger_domain::Currency::Thb {
                div { class: "mini-tax", span { class: "tax-symbol", Icon { name: "file", size: 21 } } div { strong { {crate::i18n::text("ภาษีปี {0}", &[format!("{}", year)])} } p { {crate::i18n::text("คำนวณรายปีจากรายได้และสิทธิที่ยืนยัน", &[])} } } button { class: "icon-button", "aria-label": crate::i18n::text("ดูรายละเอียดภาษี", &[]), onclick: move |_| store.page.set(Page::Tax), Icon { name: "arrow", size: 19 } } }
                }
            }
        }
        PagePanel { lazy: true, id: "overview", index: 3, selected: tab(), if !invalid_value { crate::debt_visuals::FinancialPosition { view: view.clone() } } }
        PagePanel { lazy: true, id: "overview", index: 4, selected: tab(), crate::cashflow::CashflowChart { view: view.clone() } }
        PagePanel { lazy: true, id: "overview", index: 5, selected: tab(), crate::debt_visuals::CreditDebtChart { view: view.clone() } }
        PagePanel { lazy: true, id: "overview", index: 6, selected: tab(), crate::receivables::ReceivablesChart { view: view.clone() } }
    }
}

#[component]
fn ExpenseChart(view: Dashboard) -> Element {
    let mut details = use_signal(|| None::<Option<ledger_application::ExpenseSlice>>);
    let month = match ledger_domain::Month::of(view.today) {
        Ok(m) => m,
        Err(_) => return rsx! {},
    };
    let groups = match ledger_application::expense_slices(&view, month) {
        Ok(s) => s,
        Err(e) => return rsx! { p { role: "alert", "{e}" } },
    };
    let slices = groups
        .iter()
        .map(|s| crate::pie::PieSlice {
            label: s
                .description
                .clone()
                .map(|d| {
                    if d.is_empty() {
                        crate::i18n::tr("ไม่ระบุรายละเอียด")
                    } else {
                        d
                    }
                })
                .unwrap_or_else(|| crate::i18n::tr(s.category.label())),
            amount: s.amount,
        })
        .collect();
    rsx! {
        crate::pie::PieChart { slices, label: crate::i18n::tr("รายจ่ายรวม"), onselect: move |index: usize| { if let Some(group) = groups.get(index) { details.set(Some(Some(group.clone()))); } } }
        button { class: "soft-button", "aria-haspopup": "dialog", onclick: move |_| details.set(Some(None)), {crate::i18n::tr("ดูรายการรายจ่ายทั้งหมด")} Icon { name: "arrow", size: 18 } }
        if let Some(group) = details() {
            crate::cashflow::CashflowDetailsDialog { view: view.clone(), month, initial: 0, include_pending: false, category: group.as_ref().map(|g| g.category), description: group.and_then(|g| g.description), onclose: move |_| details.set(None) }
        }
    }
}

pub fn thai_month(month: u32) -> String {
    crate::i18n::tr(match month {
        1 => "มกราคม",
        2 => "กุมภาพันธ์",
        3 => "มีนาคม",
        4 => "เมษายน",
        5 => "พฤษภาคม",
        6 => "มิถุนายน",
        7 => "กรกฎาคม",
        8 => "สิงหาคม",
        9 => "กันยายน",
        10 => "ตุลาคม",
        11 => "พฤศจิกายน",
        12 => "ธันวาคม",
        _ => "",
    })
}
