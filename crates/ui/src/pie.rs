use crate::components::money_label;
use dioxus::prelude::*;
use ledger_domain::Money;

pub(crate) const COLORS: [&str; 7] = [
    "#9C79D8", "#E999BC", "#7FBFAD", "#8BBCE0", "#E9B58D", "#B3A5D5", "#D08AA4",
];

// Filled sectors have exact hit boundaries. Dashed circles can overlap at the
// wraparound seam, causing very small slices to report their neighbour's value.
fn sector_path(offset: f64, share: f64) -> String {
    let point = |percent: f64, radius: f64| {
        let angle = percent * std::f64::consts::TAU / 100.0 - std::f64::consts::FRAC_PI_2;
        format!(
            "{} {}",
            100.0 + radius * angle.cos(),
            100.0 + radius * angle.sin()
        )
    };
    let middle = offset + share / 2.0;
    let end = offset + share;
    format!(
        "M {} A 89 89 0 0 1 {} A 89 89 0 0 1 {} L {} A 67 67 0 0 0 {} A 67 67 0 0 0 {} Z",
        point(offset, 89.0),
        point(middle, 89.0),
        point(end, 89.0),
        point(end, 67.0),
        point(middle, 67.0),
        point(offset, 67.0)
    )
}

#[derive(Clone, PartialEq)]
pub(crate) struct PieSlice {
    pub label: String,
    pub amount: Money,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use dioxus::dioxus_core::Mutation;
    use dioxus::html::{PlatformEventData, SerializedMouseData, set_event_converter};
    use std::{any::Any, cell::Cell, rc::Rc};

    fn harness() -> Element {
        let selected = use_context::<Rc<Cell<Option<usize>>>>();
        rsx! { PieChart {
            slices: vec![
                PieSlice { label: "ค่าห้อง".into(), amount: "12000".parse().expect("money") },
                PieSlice { label: "Internet".into(), amount: "3000".parse().expect("money") },
            ], label: "บิลค้าง", onselect: move |index| selected.set(Some(index)),
        } }
    }

    #[test]
    fn hovering_each_slice_shows_its_own_amount_and_share_and_click_opens_details() {
        set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let selected = Rc::new(Cell::new(None));
        let mut dom = VirtualDom::new(harness);
        dom.insert_any_root_context(Box::new(selected.clone()));
        let changes = dom.rebuild_to_vec();
        let arcs: Vec<_> = changes
            .edits
            .iter()
            .filter_map(|m| match m {
                Mutation::NewEventListener { name, id } if name == "mouseenter" => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 2);
        for (index, text, amount, share) in [
            (0, "ค่าห้อง", "12,000.00", "80.0%"),
            (1, "Internet", "3,000.00", "20.0%"),
        ] {
            let data = || {
                Event::new(
                    Rc::new(PlatformEventData::new(Box::new(
                        SerializedMouseData::default(),
                    ))) as Rc<dyn Any>,
                    true,
                )
            };
            dom.runtime()
                .handle_event("mouseenter", data(), arcs[index]);
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
            let html = dioxus_ssr::render(&dom);
            let tooltip = html
                .split("class=\"pie-tooltip\"")
                .nth(1)
                .expect("tooltip")
                .split("</div>")
                .next()
                .expect("content");
            for value in [text, amount, share, "THB"] {
                assert!(tooltip.contains(value), "{tooltip}");
            }
            dom.runtime()
                .handle_event("mouseleave", data(), arcs[index]);
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
            assert!(!dioxus_ssr::render(&dom).contains("class=\"pie-tooltip\""));
            dom.runtime().handle_event("click", data(), arcs[index]);
            assert_eq!(selected.get(), Some(index));
        }
    }
}

/// Angles are presentation only; every amount and total remains integer money.
#[component]
pub(crate) fn PieChart(
    slices: Vec<PieSlice>,
    label: String,
    onselect: EventHandler<usize>,
) -> Element {
    let mut hovered = use_signal(|| None::<usize>);
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
                div { class: "pie-wheel", onmouseleave: move |_| hovered.set(None),
                    svg { view_box: "0 0 200 200", role: "group", "aria-label": label.clone(),
                        circle { cx: "100", cy: "100", r: "78", fill: "none", stroke: "var(--surface)", stroke_width: "22" }
                        for (index, share, offset, _, _) in &arcs {
                            if *share > 0.0 {
                            path { class: "pie-slice", "data-slice": *index, "data-share": *share, "data-offset": *offset, "data-active": hovered() == Some(*index),
                                d: sector_path(*offset, *share), fill: COLORS[index % COLORS.len()],
                                role: "button", tabindex: "0", "aria-label": format!("{}: {}{} ({share:.1}%)", slices[*index].label, crate::i18n::currency_prefix(), money_label(slices[*index].amount)),
                                onmouseenter: { let index = *index; move |_| hovered.set(Some(index)) },
                                onmouseleave: move |_| hovered.set(None),
                                onfocus: { let index = *index; move |_| hovered.set(Some(index)) },
                                onblur: move |_| hovered.set(None),
                                onclick: { let index = *index; move |_| { hovered.set(None); onselect.call(index); } },
                                onkeydown: { let index = *index; move |e| match e.key() {
                                    Key::Enter => { e.prevent_default(); onselect.call(index); },
                                    Key::Character(key) if key == " " => { e.prevent_default(); onselect.call(index); },
                                    Key::Escape => hovered.set(None),
                                    _ => {},
                                } },
                            }
                            }
                        }
                    }
                    div { class: "pie-total", small { "{label}" } strong { "{money_label(total)}" } small { "{crate::i18n::currency().code()}" } }
                    if let Some(index) = hovered() && let Some(slice) = slices.get(index) {
                        div { class: "pie-tooltip", role: "tooltip",
                            strong { "{slice.label}" }
                            span { "{crate::i18n::currency_prefix()}{money_label(slice.amount)}" }
                            small { "{arcs[index].1:.1}%" }
                        }
                    }
                }
                for (index, share, _, x, _) in &arcs {
                    button { class: "pie-callout", "data-left": *x < 0.0, style: "top:{label_y[*index] / 3.0}%;", title: format!("{}: {}{} ({share:.1}%)", slices[*index].label, crate::i18n::currency_prefix(), money_label(slices[*index].amount)), onclick: { let index = *index; move |_| onselect.call(index) },
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
