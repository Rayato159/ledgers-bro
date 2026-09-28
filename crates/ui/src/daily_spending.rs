use crate::{components::money_label, i18n::tr};
use dioxus::prelude::*;
use ledger_application::{Dashboard, daily_spending};
use ledger_domain::EntryKind;

#[component]
pub(crate) fn DailySpendingChart(view: Dashboard) -> Element {
    let mut selected = use_signal(|| 6usize);
    let report = match daily_spending(&view) {
        Ok(value) => value,
        Err(_) => {
            return rsx! { section { class:"card", role:"alert", {tr("ยอดรวมเกินขอบเขตที่แสดงได้ กรุณาตรวจรายการ")} } };
        }
    };
    let day = &report.days[selected().min(6)];
    let maximum = report
        .days
        .iter()
        .map(|d| d.amount.minor())
        .max()
        .unwrap_or(0)
        .max(1);
    let mut detail = view.clone();
    detail.entries.retain(|e| {
        e.date().date() == day.date
            && !view.reversed.contains(&e.id())
            && matches!(e.kind(), EntryKind::Expense { .. })
    });
    rsx! {
        section { class:"card daily-spending", "aria-labelledby":"daily-spending-title",
            div { class:"daily-spending-heading",
                div { h2 { id:"daily-spending-title", {tr("รายจ่าย 7 วันล่าสุด")} }
                    p { class:"field-hint", "{report.days[0].date.format(\"%d/%m/%Y\")} – {report.days[6].date.format(\"%d/%m/%Y\")}" }
                }
                div { class:"daily-spending-total", small { {tr("รวม 7 วัน")} }
                    strong { "{crate::i18n::currency_prefix()}{money_label(report.total)}" }
                }
            }
            p { class:"field-hint", {tr("แตะวันที่เพื่อดูยอดและรายการ · ไม่รวมรายการยกเลิกและการโอน")} }
            div { class:"daily-spending-chart", role:"group", "aria-label":tr("รายจ่ายรายวัน"),
                for (index,item) in report.days.iter().enumerate() {
                    { let height=item.amount.minor() as f64 / maximum as f64 * 100.;
                      let label=format!("{} · {}{}",item.date,crate::i18n::currency_prefix(),money_label(item.amount));
                      rsx! {
                        button { r#type:"button", class:"daily-spending-day", "aria-label":label.clone(),
                            title:label, "aria-pressed":selected()==index, onclick:move |_|selected.set(index),
                            span { class:"daily-spending-track", "aria-hidden":"true",
                                span { class:"daily-spending-bar", style:"height:{height:.3}%;" }
                            }
                            span { class:"daily-spending-date",
                                span { "{item.date.format(\"%d\")}" }
                                span { class:"daily-spending-month", "{item.date.format(\"/%m\")}" }
                            }
                            if index==6 { small { class:"daily-spending-today", title:tr("วันนี้"), span { {tr("วันนี้")} } } } else { small { " " } }
                        }
                      }
                    }
                }
            }
            div { class:"daily-spending-detail", role:"status",
                strong { "{day.date.format(\"%d/%m/%Y\")}" }
                strong { "{crate::i18n::currency_prefix()}{money_label(day.amount)}" }
                span { {crate::i18n::text("{0} รายการ", &[day.count.to_string()])} }
            }
            if day.count==0 { p { class:"field-hint", {tr("วันนี้ไม่มีรายจ่ายที่บันทึก")} } }
            else { crate::pages::TransactionRows {view:detail,limit:usize::MAX,allow_cancel:false} }
        }
    }
}
