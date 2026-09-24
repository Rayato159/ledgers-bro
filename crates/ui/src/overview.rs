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
    let year = date.year() + 543;
    let cashflow = view
        .income
        .checked_sub(view.expenses)
        .map(money_label)
        .unwrap_or_else(|_| "—".into());
    rsx! {
        section { class: "page-heading", div { h1 { "ภาพรวม" } } span { class: "date-pill", "{date.day()} {month} {year}" } }
        div { class: "overview-top",
            section { class: "hero card",
                div { class: "hero-copy", h2 { "สินทรัพย์สุทธิ" }
                    div { class: "hero-amount", span { class: "currency", "฿" } "{money_label(view.net_worth)}" }
                    p { "รวม {view.accounts.len()} บัญชี · ข้อมูลในเครื่อง" }
                    if view.accounts.is_empty() {
                        button { class: "primary", onclick: move |_| store.account_form.set(true), Icon { name: "plus", size: 17 } "เพิ่มบัญชีแรก" }
                    } else { button { class: "soft-button", onclick: move |_| store.page.set(Page::Accounts), "ดูบัญชีของเรา" Icon { name: "arrow", size: 16 } } }
                }
                div { class: "hero-art", "aria-hidden": "true", img { src: host.art.hero.clone(), alt: "", draggable: false } }
                div { class: "hero-details",
                    div { span { class: "detail-icon", Icon { name: "wallet", size: 19 } } div { small { "สินทรัพย์" } strong { "฿{money_label(view.assets)}" } } }
                    div { span { class: "detail-icon", Icon { name: "file", size: 19 } } div { small { "หนี้สิน" } strong { "฿{money_label(view.liabilities)}" } } }
                }
            }
            div { class: "month-cards",
                section { class: "card metric income", div { class: "metric-title", span { "รายรับเดือนนี้" } span { class: "metric-icon", Icon { name: "down", size: 19 } } } strong { "฿{money_label(view.income)}" } small { "{month} {year}" } }
                section { class: "card metric expense", div { class: "metric-title", span { "รายจ่ายเดือนนี้" } span { class: "metric-icon", Icon { name: "up", size: 19 } } } strong { "฿{money_label(view.expenses)}" } small { "ไม่รวมการโอนระหว่างบัญชี" } }
                div { class: "cashflow", span { "เงินเข้า − เงินออก" } strong { "฿{cashflow}" } }
            }
        }
        crate::cashflow::CashflowChart { view: view.clone() }
        div { class: "overview-bottom",
            section { class: "card spending", div { class: "section-heading", h2 { "เงินไปไหนบ้าง" } span { class: "muted small", "เดือนนี้" } } ExpenseChart { view: view.clone() } }
            section { class: "card recent", div { class: "section-heading", h2 { "เรื่องเงินล่าสุด" } button { class: "text-button", onclick: move |_| store.page.set(Page::Transactions), "ดูทั้งหมด" Icon { name: "arrow", size: 14 } } }
                TransactionRows { view: view.clone(), limit: 4, allow_cancel: false }
                div { class: "mini-tax", span { class: "tax-symbol", Icon { name: "file", size: 21 } } div { strong { "ภาษีปี {year}" } p { "คำนวณรายปีจากรายได้และสิทธิที่ยืนยัน" } } button { class: "icon-button", "aria-label": "ดูรายละเอียดภาษี", onclick: move |_| store.page.set(Page::Tax), Icon { name: "arrow", size: 19 } } }
            }
        }
        button { class: "quick-banner", onclick: move |_| { store.page.set(Page::Chat); }, span { class: "quick-icon", Icon { name: "chat", size: 23 } } div { strong { "เล่าให้ฟัง วันนี้จ่ายอะไรไปบ้าง?" } p { "ลองพิมพ์ กาแฟ 80 แล้วเลือกบัญชีและหมวด" } } span { class: "round-arrow", Icon { name: "arrow", size: 22 } } }
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
            div { class: "donut", svg { view_box: "0 0 160 160", role: "img", "aria-label": "กราฟสัดส่วนรายจ่าย รายละเอียดอยู่ในรายการข้างกราฟ",
                circle { cx: "80", cy: "80", r: "62", fill: "none", stroke: "var(--surface)", stroke_width: "18" }
                for (_, _, percentage, offset, color) in &arcs {
                    circle { cx: "80", cy: "80", r: "62", fill: "none", stroke: *color, stroke_width: "18", path_length: "100", stroke_dasharray: "{percentage} {100.0 - percentage}", stroke_dashoffset: "{-offset}", transform: "rotate(-90 80 80)" }
                }
            } div { class: "donut-label", small { "รายจ่ายรวม" } strong { "฿{money_label(view.expenses)}" } } }
            div { class: "chart-legend",
                if arcs.is_empty() { p { class: "muted", "ยังไม่มีรายจ่ายเดือนนี้\nบันทึกครั้งแรก กราฟจะเริ่มเล่าเรื่องให้เรา" } }
                for (category, amount, percentage, _, color) in arcs {
                    div { class: "legend-item", span { class: "legend-art", style: "border-color:{color}", ArtIcon { name: category_art(category), size: 28 } } span { "{category.label()}" } strong { "{money_label(amount)}" } small { "{percentage:.0}%" } }
                }
            }
        }
    }
}

pub fn thai_month(month: u32) -> &'static str {
    match month {
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
    }
}
