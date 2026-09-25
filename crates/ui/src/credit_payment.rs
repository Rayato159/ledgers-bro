//! Presentation for an explicit card settlement; accounting stays in application.
use crate::{components::money_label, i18n::text, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;

#[component]
pub(crate) fn CreditPaymentFields(
    input: EntryInput,
    view: Dashboard,
    mut full: Signal<bool>,
) -> Element {
    let mut store = use_context::<UiState>();
    let cards = match credit_cards(&view) {
        Ok(cards) => cards,
        Err(error) => return rsx! { p { role: "alert", "{error}" } },
    };
    let selected = cards
        .iter()
        .find(|c| Some(c.account.id()) == input.destination);
    let remaining = selected.map(|c| c.outstanding);
    let for_select = view.clone();
    let for_full = view.clone();
    rsx! {
        section { class: "credit-payment-picker", "aria-label": text("ชำระบัตรเครดิต", &[]),
            label { r#for: "payment-card", {text("เลือกบัตรที่ต้องการชำระ", &[])} }
            select { id: "payment-card", required: true, disabled: *store.busy.read(), value: input.destination.map(|id| id.to_string()).unwrap_or_default(),
                onchange: move |event| {
                    if let Ok(id) = event.value().parse() {
                        let current = store.input.peek().clone();
                        if let Some(current) = current {
                            match select_credit_payment(&for_select, &current, id) {
                                Ok(updated) => { full.set(true); store.update_entry(|i| *i = updated); }
                                Err(error) => store.notice.set(Some((true, error.to_string()))),
                            }
                        }
                    } else { store.update_entry(|i| { i.destination = None; i.amount.clear(); }); }
                },
                option { value: "", selected: input.destination.is_none(), {text("เลือกบัตรเครดิต", &[])} }
                for card in cards.iter().filter(|c| !c.account.is_archived() && c.outstanding > ledger_domain::Money::ZERO) {
                    option { value: "{card.account.id()}", selected: input.destination == Some(card.account.id()), "{card.account.name().as_str()} · {crate::i18n::currency_prefix()}{money_label(card.outstanding)}" }
                }
            }
            if !cards.iter().any(|c| !c.account.is_archived() && c.outstanding > ledger_domain::Money::ZERO) {
                p { class: "field-hint", {text("ยังไม่มีบัตรที่มียอดค้างให้ชำระ เพิ่มบัญชีบัตรหรือบันทึกรายจ่ายผ่านบัตรก่อน", &[])} }
            }
            if let Some(remaining) = remaining {
                div { class: "credit-payment-balance", span { {text("ยอดค้างของบัตรนี้", &[])} } strong { "{crate::i18n::currency_prefix()}{money_label(remaining)}" } }
            }
            div { class: "payment-amount-options", role: "group", "aria-label": text("จำนวนที่ต้องการชำระ", &[]),
                button { r#type: "button", class: if full() { "soft-button selected" } else { "soft-button" }, "aria-pressed": full(), disabled: *store.busy.read() || remaining.is_none(), onclick: move |_| {
                    let current = store.input.peek().clone();
                    if let Some(current) = current && let Some(id) = current.destination {
                        match select_credit_payment(&for_full, &current, id) {
                            Ok(updated) => { full.set(true); store.update_entry(|i| *i = updated); }
                            Err(error) => store.notice.set(Some((true, error.to_string()))),
                        }
                    }
                }, {text("เต็มจำนวน", &[])} }
                button { r#type: "button", class: if !full() { "soft-button selected" } else { "soft-button" }, "aria-pressed": !full(), disabled: *store.busy.read() || remaining.is_none(), onclick: move |_| {
                    if full() { full.set(false); store.update_entry(|i| i.amount.clear()); }
                }, {text("บางส่วน", &[])} }
            }
            p { class: "field-hint", {text("เต็มจำนวนคือหนี้คงค้างทั้งหมด รวมยอดยกมาและรอบที่ยังไม่ตัดบิล · จ่ายบางส่วนได้โดยกรอกยอดด้านล่าง", &[])} }
            p { class: "field-hint", {text("ลดเงินในบัญชีที่จ่ายและลดหนี้บัตร ไม่เพิ่มรายจ่ายซ้ำ", &[])} }
        }
    }
}
