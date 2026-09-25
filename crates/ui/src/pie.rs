use crate::components::money_label;
use dioxus::prelude::*;
use ledger_domain::Money;

pub(crate) const COLORS: [&str; 7] = [
    "#9C79D8", "#E999BC", "#7FBFAD", "#8BBCE0", "#E9B58D", "#B3A5D5", "#D08AA4",
];

#[derive(Clone, PartialEq)]
pub(crate) struct PieSlice {
    pub label: String,
    pub amount: Money,
}

/// Angles are presentation only; every amount and total remains integer money.
#[component]
pub(crate) fn PieChart(
    slices: Vec<PieSlice>,
    label: String,
    onselect: EventHandler<usize>,
) -> Element {
    let total = slices
        .iter()
        .try_fold(Money::ZERO, |sum, s| sum.checked_add(s.amount));
    let Ok(total) = total else {
        return rsx! { p { role: "alert", {crate::i18n::tr("ยอดรวมเกินขอบเขตที่แสดงได้ กรุณาตรวจรายการ")} } };
    };
    let mut offset = 0.0;
    let arcs: Vec<_> = slices
        .iter()
        .enumerate()
        .map(|(index, s)| {
            let share = if total.minor() > 0 {
                s.amount.minor() as f64 / total.minor() as f64 * 100.0
            } else {
                0.0
            };
            let angle = (offset + share / 2.0) * std::f64::consts::TAU / 100.0
                - std::f64::consts::FRAC_PI_2;
            let arc = (index, share, offset, angle.cos(), angle.sin());
            offset += share;
            arc
        })
        .collect();
    let mut label_y = vec![150.0; slices.len()];
    for left in [true, false] {
        let mut side: Vec<_> = arcs
            .iter()
            .filter(|(_, _, _, x, _)| (*x < 0.0) == left)
            .collect();
        side.sort_by(|a, b| a.4.total_cmp(&b.4));
        let count = side.len();
        for (row, arc) in side.into_iter().enumerate() {
            label_y[arc.0] = if count == 1 {
                150.0
            } else {
                45.0 + row as f64 * 210.0 / (count - 1) as f64
            };
        }
    }
    rsx! {
        div { class: "pie-chart", "data-orbit": !slices.is_empty() && slices.len() <= 6,
            div { class: "pie-orbit",
                svg { class: "pie-leaders", view_box: "0 0 600 300", "aria-hidden": "true",
                    for (index, _, _, x, y) in &arcs {
                        path { d: format!("M {} {} L {} {} L {} {}", 300.0 + x * 105.0, 150.0 + y * 105.0, if *x < 0.0 { 175.0 } else { 425.0 }, label_y[*index], if *x < 0.0 { 150.0 } else { 450.0 }, label_y[*index]), stroke: COLORS[index % COLORS.len()], stroke_width: "1.5", fill: "none" }
                    }
                }
                div { class: "pie-wheel",
                    svg { view_box: "0 0 200 200", role: "img", "aria-label": label.clone(),
                        circle { cx: "100", cy: "100", r: "78", fill: "none", stroke: "var(--surface)", stroke_width: "22" }
                        for (index, share, offset, _, _) in &arcs {
                            circle { cx: "100", cy: "100", r: "78", fill: "none", stroke: COLORS[index % COLORS.len()], stroke_width: "22", path_length: "100", stroke_dasharray: "{share} {100.0 - share}", stroke_dashoffset: "{-offset}", transform: "rotate(-90 100 100)",
                                title { "{slices[*index].label}: {money_label(slices[*index].amount)} ({share:.1}%)" }
                            }
                        }
                    }
                    div { class: "pie-total", small { "{label}" } strong { "{money_label(total)}" } small { "{crate::i18n::currency().code()}" } }
                }
                for (index, share, _, x, _) in &arcs {
                    button { class: "pie-callout", "data-left": *x < 0.0, style: "top:{label_y[*index] / 3.0}%;", title: slices[*index].label.clone(), onclick: { let index = *index; move |_| onselect.call(index) },
                        strong { "{slices[*index].label}" } span { "{money_label(slices[*index].amount)}" } small { "{share:.1}%" }
                    }
                }
            }
            div { class: "pie-legend", for (index, share, _, _, _) in &arcs {
                button { class: "pie-legend-row", onclick: { let index = *index; move |_| onselect.call(index) },
                    span { class: "expense-color", style: "background:{COLORS[index % COLORS.len()]}" }
                    span { "{slices[*index].label}" } strong { "{money_label(slices[*index].amount)}" } small { "{share:.1}%" }
                }
            } }
            if slices.is_empty() { p { class: "muted", {crate::i18n::tr("ไม่มีรายการในเดือนนี้")} } }
        }
    }
}
