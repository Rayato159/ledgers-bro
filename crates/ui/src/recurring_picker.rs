use crate::{components::money_label, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

pub(crate) fn installment_label(schedule: &RecurringExpense, month: Month) -> String {
    let number = schedule
        .due()
        .number_in(month)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    match schedule.due().installments() {
        Some(total) => crate::i18n::text(
            "งวดที่ {0}/{1} · {2}",
            &[number.to_string(), total.to_string(), month.to_string()],
        ),
        None => crate::i18n::text(
            "งวดที่ {0} · {1} · ไม่กำหนดจำนวนงวด",
            &[number.to_string(), month.to_string()],
        ),
    }
}

/// Shared by manual entries and every row of a prompt batch.
#[component]
pub(crate) fn RecurringPicker(
    id: String,
    input: EntryInput,
    view: Dashboard,
    onchange: EventHandler<EntryInput>,
) -> Element {
    let store = use_context::<UiState>();
    let options = recurring_progress(&view);
    let selection = input.recurring.clone();
    let selected_schedule = selection
        .as_ref()
        .and_then(|s| view.recurring.iter().find(|p| p.id() == s.recurring))
        .cloned();
    let selection_valid = selection.as_ref().is_some_and(|s| {
        options
            .iter()
            .any(|p| p.schedule.id() == s.recurring && p.next_unpaid.is_some())
    });
    let for_select = input.clone();
    let select_options = options.clone();
    rsx! {
        div { class: "recurring-picker",
            label { r#for: "{id}-plan", {crate::i18n::text("ผูกกับหนี้ / รายจ่ายประจำ (ไม่จำเป็น)", &[])} }
            select { id: "{id}-plan", disabled: *store.busy.read(), value: selection.as_ref().map(|s| s.recurring.to_string()).unwrap_or_default(),
                onchange: move |e| {
                    let mut updated = for_select.clone();
                    if let Some(progress) = select_options.iter().find(|p| p.schedule.id().to_string() == e.value()) && let Some(month) = progress.next_unpaid {
                        updated = select_recurring(&updated, &progress.schedule, month);
                    } else { updated.recurring = None; }
                    onchange.call(updated);
                },
                option { value: "", selected: selection.is_none(), {crate::i18n::text("รายการทั่วไป — ไม่ผูกกับแผน", &[])} }
                for progress in options.iter().filter(|p| p.next_unpaid.is_some()) {
                    if let Some(month) = progress.next_unpaid {
                        option { value: "{progress.schedule.id()}", selected: selection.as_ref().is_some_and(|s| s.recurring == progress.schedule.id()), "{progress.schedule.name().as_str()} · {installment_label(&progress.schedule, month)} · {crate::i18n::currency_prefix()}{money_label(progress.schedule.amount().money())}" }
                    }
                }
                if let Some(s) = &selection && !selection_valid {
                    option { value: "{s.recurring}", selected: true, {crate::i18n::text("แผนที่เลือกเปลี่ยนไปหรือจ่ายครบแล้ว — กรุณาเลือกใหม่", &[])} }
                }
            }
            if let (Some(selection), Some(schedule)) = (selection.clone(), selected_schedule) {
                label { r#for: "{id}-period", {crate::i18n::text("งวดที่จะจ่าย (ค.ศ.)", &[])} }
                input { id: "{id}-period", r#type: "month", value: "{selection.month}", min: "{schedule.due().start()}", disabled: *store.busy.read(),
                    onchange: move |e| {
                        let mut updated = input.clone();
                        updated.recurring = Some(RecurringSelection { recurring: selection.recurring, month: e.value() });
                        onchange.call(updated);
                    }
                }
                p { class: "field-hint", {crate::i18n::text("จะนับว่าจ่ายครบหนึ่งงวดเมื่อยืนยันบันทึก · เติมเฉพาะช่องว่างจากแผน ยอดและวันที่ที่กรอกแล้วคงเดิม · แก้งวดได้หากจ่ายล่วงหน้าหรือย้อนหลัง", &[])} }
            }
        }
    }
}
