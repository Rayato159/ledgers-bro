use crate::{components::money_label, overview::thai_month};
use dioxus::prelude::*;
use ledger_application::{Dashboard, FlowHealth, monthly_cashflow};

#[component]
pub(crate) fn CashflowChart(view: Dashboard) -> Element {
    let mut selected = use_signal(|| 5_usize);
    let periods = match monthly_cashflow(&view) {
        Ok(periods) => periods,
        Err(_) => {
            return rsx! { section { class: "card", role: "alert", "ยอดรวมเกินขอบเขตที่แสดงได้ กรุณาตรวจรายการ" } };
        }
    };
    let period = &periods[selected().min(5)];
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
        "ยังไม่มีรายรับหรือรายจ่ายในเดือนนี้".to_owned()
    } else if period.income == period.expenses {
        "รายรับเท่ากับรายจ่าย · ต่างกัน 0%".into()
    } else {
        let (larger, smaller) = if period.income > period.expenses {
            ("รายรับ", "รายจ่าย")
        } else {
            ("รายจ่าย", "รายรับ")
        };
        match period.difference_percent() {
            Some(percent) => format!(
                "{larger}มากกว่า{smaller} {}.{:02}% (เทียบกับ{smaller})",
                percent / 100,
                percent % 100
            ),
            None => format!("มีเฉพาะ{larger} · {smaller}เป็นศูนย์ จึงเทียบเป็นเปอร์เซ็นต์ไม่ได้"),
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
            div { class: "section-heading", div { h2 { id: "flow-title", "เงินเข้าเทียบเงินออก" } p { class: "muted small", "ย้อนหลัง 6 เดือน · แตะเดือนเพื่อดูรายละเอียด" } }
                span { class: "flow-health {class}", "{health}" }
            }
            div { class: "flow-layout",
                div {
                    div { class: "flow-legend", span { class: "flow-income-key", "รายรับ" } span { class: "flow-expense-key", "รายจ่าย" } }
                    div { class: "flow-bars", role: "group", "aria-label": "กราฟรายรับและรายจ่ายรายเดือน",
                        for (index, month) in periods.iter().enumerate() {
                            { let incoming = month.income.minor() as f64 / maximum as f64 * 100.0;
                              let outgoing = month.expenses.minor() as f64 / maximum as f64 * 100.0;
                              rsx! {
                                button { r#type: "button", class: if index == selected() { "flow-month selected" } else { "flow-month" }, "aria-pressed": index == selected(),
                                    "aria-label": "{thai_month(month.month)} {month.year + 543} รายรับ {money_label(month.income)} บาท รายจ่าย {money_label(month.expenses)} บาท",
                                    onclick: move |_| selected.set(index),
                                    span { class: "flow-bar-pair", "aria-hidden": "true",
                                        span { class: "flow-bar incoming", style: "height: {incoming}%" }
                                        span { class: "flow-bar outgoing", style: "height: {outgoing}%" }
                                    }
                                    span { class: "flow-month-label", "{month.month}/{(month.year + 543) % 100}" }
                                }
                              }
                            }
                        }
                    }
                    p { class: "field-hint", "นับตามวันที่รายการ ถึงวันที่ {view.today} · ไม่รวมยอดเริ่มต้น เงินโอน และรายการที่ยกเลิก" }
                }
                div { class: "flow-details", "aria-live": "polite",
                    h3 { "{thai_month(period.month)} {period.year + 543}" }
                    div { class: "flow-amounts",
                        div { small { "รายรับ" } strong { "฿{money_label(period.income)}" } span { "{income_share:.1}% ของยอดรวม" } }
                        div { small { "รายจ่าย" } strong { "฿{money_label(period.expenses)}" } span { "{expense_share:.1}% ของยอดรวม" } }
                    }
                    p { "{comparison}" }
                    p { class: "flow-net", "เงินเหลือสุทธิ ฿{net}" }
                    div { class: "flow-rate-heading", span { "Flow rate" } strong { "{rate_label}" } }
                    div { class: "flow-gauge", role: "img", "aria-label": "Flow rate {rate_label} ช่วงลบหนึ่งถึงบวกหนึ่ง",
                        if rate.is_some() { span { class: "flow-pointer", style: "left: {position}%" } }
                    }
                    div { class: "flow-scale", span { "−1.0" } span { "0" } span { "+1.0" } }
                    p { "{advice}" }
                }
            }
            details { class: "flow-explanation", summary { "สูตรและเกณฑ์สุขภาพการเงิน" }
                p { "Flow rate = (รายรับ − รายจ่าย) ÷ (รายรับ + รายจ่าย) มีค่าตั้งแต่ −1 ถึง +1 ถ้าทั้งคู่เป็นศูนย์จะแสดงว่ายังไม่มีข้อมูล" }
                p { "แย่: ต่ำกว่า 0 · พอใช้: ตั้งแต่ 0 แต่น้อยกว่า 0.10 · ดี: ตั้งแต่ 0.10 ขึ้นไป เป็นเกณฑ์ของแอปสำหรับรายรับ–รายจ่ายที่บันทึก ไม่ได้ประเมินภาระหนี้ เงินสำรอง หรือข้อมูลที่ยังไม่ได้จด" }
                p { "ตัวอย่าง รายรับ 30,000 รายจ่าย 20,000 → Flow rate +0.200 และรายรับมากกว่ารายจ่าย 50% การรูดบัตรนับเป็นรายจ่ายวันที่รูด ส่วนโอนจ่ายบัตรไม่นับซ้ำ" }
            }
        }
    }
}
