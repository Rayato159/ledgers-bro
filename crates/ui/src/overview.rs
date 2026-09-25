use crate::artwork::*;
use crate::{HostInfo, components::*, pages::TransactionRows, state::*};
use chrono::Datelike;
use dioxus::prelude::*;
use ledger_application::Dashboard;

#[component]
pub fn Overview(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
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
        div { class: "overview-top",
            section { class: "hero card",
                div { class: "hero-copy", h2 { {crate::i18n::text("สินทรัพย์สุทธิ", &[])} }
                    div { class: "hero-amount", span { class: "currency", "{crate::i18n::currency().code()}" } "{money_label(view.net_worth)}" }
                    p { {crate::i18n::text("รวม {0} บัญชี · ข้อมูลในเครื่อง", &[format!("{}", view.accounts.len())])} }
                    if view.accounts.is_empty() {
                        button { class: "primary", onclick: move |_| store.account_form.set(true), Icon { name: "plus", size: 17 } {crate::i18n::text("เพิ่มบัญชีแรก", &[])} }
                    } else { button { class: "soft-button", onclick: move |_| store.page.set(Page::Accounts), {crate::i18n::text("ดูบัญชีของเรา", &[])} Icon { name: "arrow", size: 16 } } }
                }
                div { class: "hero-art", "aria-hidden": "true", img { src: host.art.hero.clone(), alt: "", draggable: false } }
                div { class: "hero-details",
                    div { span { class: "detail-icon", Icon { name: "wallet", size: 19 } } div { small { {crate::i18n::text("สินทรัพย์", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(view.assets)}" } } }
                    div { span { class: "detail-icon", Icon { name: "file", size: 19 } } div { small { {crate::i18n::text("หนี้สิน", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(view.liabilities)}" } } }
                }
            }
            div { class: "month-cards",
                section { class: "card metric income", div { class: "metric-title", span { {crate::i18n::text("รายรับเดือนนี้", &[])} } span { class: "metric-icon", Icon { name: "down", size: 19 } } } strong { "{crate::i18n::currency_prefix()}{money_label(view.income)}" } small { "{month} {year}" } }
                section { class: "card metric expense", div { class: "metric-title", span { {crate::i18n::text("รายจ่ายเดือนนี้", &[])} } span { class: "metric-icon", Icon { name: "up", size: 19 } } } strong { "{crate::i18n::currency_prefix()}{money_label(view.expenses)}" } small { {crate::i18n::text("ไม่รวมการโอนระหว่างบัญชี", &[])} } }
                div { class: "cashflow", span { {crate::i18n::text("รายรับ − รายจ่ายที่บันทึก", &[])} } strong { "{crate::i18n::currency_prefix()}{cashflow}" } }
            }
        }
        crate::debt_visuals::FinancialPosition { view: view.clone() }
        crate::debt_visuals::CreditDebtChart { view: view.clone() }
        crate::cashflow::CashflowChart { view: view.clone() }
        crate::receivables::ReceivablesChart { view: view.clone() }
        div { class: "overview-bottom",
            section { class: "card spending", div { class: "section-heading", h2 { {crate::i18n::text("เงินไปไหนบ้าง", &[])} } span { class: "muted small", {crate::i18n::text("เดือนนี้", &[])} } } ExpenseChart { view: view.clone() } }
            section { class: "card recent", div { class: "section-heading", h2 { {crate::i18n::text("เรื่องเงินล่าสุด", &[])} } button { class: "text-button", onclick: move |_| store.page.set(Page::Transactions), {crate::i18n::text("ดูทั้งหมด", &[])} Icon { name: "arrow", size: 14 } } }
                TransactionRows { view: view.clone(), limit: 4, allow_cancel: false }
                if view.thai_tax_enabled && view.currency == ledger_domain::Currency::Thb {
                div { class: "mini-tax", span { class: "tax-symbol", Icon { name: "file", size: 21 } } div { strong { {crate::i18n::text("ภาษีปี {0}", &[format!("{}", year)])} } p { {crate::i18n::text("คำนวณรายปีจากรายได้และสิทธิที่ยืนยัน", &[])} } } button { class: "icon-button", "aria-label": crate::i18n::text("ดูรายละเอียดภาษี", &[]), onclick: move |_| store.page.set(Page::Tax), Icon { name: "arrow", size: 19 } } }
                }
            }
        }
        button { class: "quick-banner", onclick: move |_| { store.page.set(Page::Chat); }, span { class: "quick-icon", Icon { name: "chat", size: 23 } } div { strong { {crate::i18n::text("เล่าให้ฟัง วันนี้จ่ายอะไรไปบ้าง?", &[])} } p { {crate::i18n::text("ลองพิมพ์ กาแฟ 80 แล้วเลือกบัญชีและหมวด", &[])} } } span { class: "round-arrow", Icon { name: "arrow", size: 22 } } }
    }
}

#[component]
fn ExpenseChart(view: Dashboard) -> Element {
    const COLORS: [&str; 7] = [
        "#9C79D8", "#E999BC", "#7FBFAD", "#8BBCE0", "#E9B58D", "#B3A5D5", "#D08AA4",
    ];
    let total = view.expenses.minor();
    let mut offset = 0.0;
    let arcs: Vec<_> = view
        .category_expenses
        .iter()
        .enumerate()
        .map(|(index, (category, amount))| {
            // Floating point is only used for SVG geometry, never accounting.
            let percentage = if total > 0 {
                amount.minor() as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            let arc = (
                *category,
                *amount,
                percentage,
                offset,
                COLORS[index % COLORS.len()],
            );
            offset += percentage;
            arc
        })
        .collect();
    rsx! {
        div { class: "chart-layout",
            div { class: "donut", svg { view_box: "0 0 160 160", role: "img", "aria-label": crate::i18n::text("กราฟสัดส่วนรายจ่าย รายละเอียดอยู่ในรายการข้างกราฟ", &[]),
                circle { cx: "80", cy: "80", r: "62", fill: "none", stroke: "var(--surface)", stroke_width: "18" }
                for (_, _, percentage, offset, color) in &arcs {
                    circle { cx: "80", cy: "80", r: "62", fill: "none", stroke: *color, stroke_width: "18", path_length: "100", stroke_dasharray: "{percentage} {100.0 - percentage}", stroke_dashoffset: "{-offset}", transform: "rotate(-90 80 80)" }
                }
            } div { class: "donut-label", small { {crate::i18n::text("รายจ่ายรวม", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(view.expenses)}" } } }
            div { class: "chart-legend",
                if arcs.is_empty() { p { class: "muted", {crate::i18n::text("ยังไม่มีรายจ่ายเดือนนี้\nบันทึกครั้งแรก กราฟจะเริ่มเล่าเรื่องให้เรา", &[])} } }
                for (category, amount, percentage, _, color) in arcs {
                    div { class: "legend-item", span { class: "legend-art", style: "border-color:{color}", ArtIcon { name: category_art(category), size: 28 } } span { "{crate::i18n::tr(category.label())}" } strong { "{money_label(amount)}" } small { "{percentage:.0}%" } }
                }
            }
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
