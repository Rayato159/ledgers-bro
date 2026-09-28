use crate::{
    components::Icon,
    components::money_label,
    i18n::{text, tr},
};
use chrono::Datelike;
use dioxus::prelude::*;
use ledger_application::{Dashboard, RecurringOccurrence};
use ledger_domain::{EntryDate, Month, RecurringExpense};

// Keep endpoints inset so the day markers stay inside the chart at every width.
fn day_position(day: u32, last: u32) -> f64 {
    6.0 + f64::from(day.saturating_sub(1)) / f64::from(last.saturating_sub(1).max(1)) * 88.0
}

fn today_position(month: Month, today: EntryDate, last: u32) -> Option<f64> {
    (month.key() == today.month_key()).then(|| day_position(today.date().day(), last))
}

fn tone(item: &RecurringOccurrence, today: EntryDate) -> &'static str {
    if item.paid_entry.is_some() {
        "paid"
    } else if item.due < today {
        "overdue"
    } else {
        "pending"
    }
}

#[component]
pub(super) fn BillTimeline(
    month: Month,
    view: Dashboard,
    items: Vec<RecurringOccurrence>,
    onselect: EventHandler<RecurringExpense>,
) -> Element {
    let Ok(last_date) = month.on_day(31) else {
        return rsx! {};
    };
    let last = last_date.date().day();
    let today = today_position(month, view.today, last);
    let period_note = if today.is_some() {
        text(
            "วันนี้ {0} · วันที่ {1} จาก {2} วัน",
            &[
                view.today.to_string(),
                view.today.date().day().to_string(),
                last.to_string(),
            ],
        )
    } else if month.key() < view.today.month_key() {
        tr("เดือนนี้ผ่านไปแล้ว")
    } else {
        tr("เดือนนี้ยังมาไม่ถึง")
    };
    rsx! {
        section { class: "bill-timeline", "aria-label": tr("เส้นเวลาบิลรายเดือน"),
            div { class: "bill-timeline-heading",
                div { h3 { {tr("เส้นเวลาบิลรายเดือน")} } p { class: "muted", "{period_note}" } }
                div { class: "bill-timeline-legend",
                    for (color, label) in [("paid", "บันทึกแล้ว"), ("pending", "ยังไม่จ่าย"), ("overdue", "เลยกำหนด")] {
                        span { "data-tone": color, i { "aria-hidden": "true" } {tr(label)} }
                    }
                }
            }
            p { class: "field-hint", {tr("จุดคือวันครบกำหนด ไม่ใช่วันที่จ่ายจริง · กดรายการเพื่อดูรายละเอียด · ลงบัตรแล้วยังต้องชำระบัตร")} }
            div { class: "bill-timeline-scroll", tabindex: "0", role: "region", "aria-label": tr("รายการตามวันครบกำหนด"),
                div { class: "bill-timeline-axis", "aria-hidden": "true",
                    span { {tr("วันครบกำหนด")} }
                    div { class: "bill-timeline-track",
                        for day in [1, 8, 15, 22, last] {
                            span { class: "bill-axis-day", style: "left:{day_position(day, last)}%;", "{day}" }
                        }
                        if let Some(position) = today { span { class: "bill-today-label", style: "left:{position}%;", {tr("วันนี้")} } }
                    }
                }
                for item in items {
                    { let status = super::occurrence_status(&item, &view);
                      let color = tone(&item, view.today);
                      let day = item.due.date().day();
                      let amount = if item.paid_entry.is_some() { item.paid_amount } else { item.schedule.amount().money() };
                      let detail = format!("{} · {} · {}{} · {}", item.schedule.name().as_str(), item.due, crate::i18n::currency_prefix(), money_label(amount), status);
                      rsx! {
                        button { class: "bill-timeline-row", r#type: "button", "data-tone": color, "data-plan": "{item.schedule.id()}",
                            title: detail.clone(), "aria-label": detail, "aria-haspopup": "dialog",
                            onclick: move |_| onselect.call(item.schedule.clone()),
                            span { class: "bill-timeline-copy",
                                strong { "{item.schedule.name().as_str()}" }
                                span { "{crate::i18n::currency_prefix()}{money_label(amount)}" }
                                small { "{status}" }
                            }
                            span { class: "bill-timeline-track", "aria-hidden": "true",
                                span { class: "bill-timeline-rail" }
                                if let Some(position) = today { span { class: "bill-today-line", style: "left:{position}%;" } }
                                span { class: "bill-due-marker", style: "left:{day_position(day, last)}%;",
                                    if color == "paid" { Icon { name: "check", size: 16 } }
                                    else { "{day}" }
                                }
                            }
                        }
                      }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn calendar_positions_use_real_month_lengths_and_hide_today_outside_its_month() {
        for (month, end) in [
            ("2028-02", 29),
            ("2026-02", 28),
            ("2026-04", 30),
            ("2026-12", 31),
        ] {
            let month: Month = month.parse().expect("month");
            let last = month.on_day(31).expect("end");
            assert_eq!(last.date().day(), end);
            assert_eq!(day_position(1, end), 6.0);
            assert_eq!(today_position(month, last, end), Some(94.0));
            assert_eq!(
                today_position(month.shifted(-1).expect("before"), last, end),
                None
            );
            assert_eq!(
                today_position(month.shifted(1).expect("after"), last, end),
                None
            );
        }
    }

    #[test]
    fn timeline_follows_settlements_and_due_dates_including_due_today() {
        let mut state = super::super::tests::fixture();
        let today = "2026-10-16".parse().expect("today");
        let view = ledger_application::dashboard(state.clone(), today).expect("view");
        let mut item =
            ledger_application::recurring_month(&view, "2026-10".parse().expect("month"))
                .expect("month")
                .items
                .remove(0);
        assert_eq!(tone(&item, today), "pending");
        assert_eq!(
            tone(&item, "2026-10-17".parse().expect("tomorrow")),
            "overdue"
        );
        item.paid_entry = Some(
            "00000000-0000-0000-0000-000000000005"
                .parse()
                .expect("entry"),
        );
        assert_eq!(tone(&item, today), "paid");
        // The application projection, not a stored settlement flag, controls the timeline.
        state.entries.clear();
        state
            .settlements
            .push(ledger_application::RecurringSettlement {
                recurring: item.schedule.id(),
                month: "2026-10".parse().expect("month"),
                entry: item.paid_entry.expect("entry"),
            });
        let view = ledger_application::dashboard(state, today).expect("view");
        let item = ledger_application::recurring_month(&view, "2026-10".parse().expect("month"))
            .expect("month")
            .items
            .remove(0);
        assert_eq!(tone(&item, today), "pending");
    }
}
