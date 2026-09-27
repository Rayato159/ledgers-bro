use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;
use std::collections::BTreeMap;

// A check approves the exact saved input; editing invalidates that approval.
type ReviewedCards = Signal<BTreeMap<usize, EntryInput>>;

fn edit_input(mut editing: Signal<Option<EntryInput>>, apply: impl FnOnce(&mut EntryInput)) {
    if let Some(input) = editing.write().as_mut() {
        let amount = input.amount.clone();
        apply(input);
        if amount != input.amount
            && let Some(receipt) = &mut input.receipt
        {
            receipt.reviewed = false;
        }
    }
}

#[component]
pub(crate) fn BatchReview(source: String, drafts: Vec<ModelDraft>, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let reviewed: ReviewedCards = use_signal(BTreeMap::new);
    let checked = drafts
        .iter()
        .enumerate()
        .filter(|(index, draft)| reviewed.read().get(index) == Some(&draft.input))
        .count();
    let ready = !drafts.is_empty() && checked == drafts.len();
    let confirm_id = "batch-confirm";
    rsx! {
        div { class: "review-intro",
            h3 { {crate::i18n::text("ตรวจ {0} รายการก่อนบันทึก", &[drafts.len().to_string()])} }
            p { class: "field-hint", {crate::i18n::tr("ตรวจรายละเอียดแล้วติ๊กแต่ละรายการ หรือแก้ไขในบัตร จากนั้นยืนยันบันทึกทั้งชุด")} }
        }
        details { class: "batch-source", summary { {crate::i18n::tr("ดูข้อความต้นฉบับ")} } pre { class: "model-source", "{source}" } }
        div { class: "review-cards", for (index, draft) in drafts.iter().enumerate() {
            ReviewCard { key: "{index}", index, draft: draft.clone(), view: view.clone(), reviewed }
        } }
        div { class: "review-footer",
            div {
                strong { role: "status", {crate::i18n::text("ตรวจแล้ว {0} / {1} รายการ", &[checked.to_string(), drafts.len().to_string()])} }
                p { class: "field-hint", {crate::i18n::tr("ยังไม่มีรายการถูกบันทึก จนกว่าจะยืนยันทั้งชุด")} }
            }
            button { id: confirm_id, class: "primary", disabled: !ready || *store.busy.read(), onclick: move |_| { if ready { store.commit_reviewed_batch(); } },
                Icon { name: "check", size: 18 }
                {crate::i18n::text("ยืนยันบันทึกทั้งหมด {0} รายการ", &[drafts.len().to_string()])}
            }
        }
        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.account_form.set(true), {crate::i18n::tr("เพิ่มบัญชีที่ยังไม่มี")} }
    }
}

#[cfg(test)]
#[path = "batch_entry_tests.rs"]
mod tests;

#[component]
fn ReviewCard(
    index: usize,
    draft: ModelDraft,
    view: Dashboard,
    mut reviewed: ReviewedCards,
) -> Element {
    let mut store = use_context::<UiState>();
    let initial = draft.input.clone();
    let mut editing =
        use_signal(move || (!missing_entry_fields(&initial).is_empty()).then_some(initial));
    let mut error = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);
    use_drop(move || {
        // A navigation away cancels the card's read-only preview task.
        if *saving.peek() {
            store.busy.set(false);
        }
    });
    let checked = reviewed.read().get(&index) == Some(&draft.input) && editing.read().is_none();
    let missing = missing_entry_fields(&draft.input);
    let kind = match draft.input.kind {
        TransactionKind::Expense => "รายจ่าย",
        TransactionKind::Income => "รายรับ",
        TransactionKind::Transfer => "โอนเงิน",
    };
    let title = if draft.input.note.trim().is_empty() {
        crate::i18n::tr("ยังไม่มีรายละเอียด · กดแก้ไขเพื่อระบุรายการ")
    } else {
        draft.input.note.clone()
    };
    let save = move |_| {
        let Some(input) = editing.peek().clone() else {
            return;
        };
        if *store.busy.peek() {
            return;
        }
        let fields = missing_entry_fields(&input);
        if !fields.is_empty() {
            error.set(Some(crate::i18n::text(
                "กรุณาเติมข้อมูลก่อนบันทึก: {0}",
                &[crate::i18n::field_list(&fields)],
            )));
            return;
        }
        store.busy.set(true);
        saving.set(true);
        error.set(None);
        let gateway = store.gateway.peek().clone();
        // Preview validates but does not write. The final batch commit is atomic.
        spawn(async move {
            match gateway.0.request(Command::Preview(input.clone())).await {
                Ok(Response::Prepared(_)) => {
                    if let Some((_, drafts)) = store.batch.write().as_mut()
                        && let Some(draft) = drafts.get_mut(index)
                    {
                        draft.input = input.clone();
                        draft.guidance.clear();
                    }
                    store.batch_prepared.set(None);
                    reviewed.write().insert(index, input);
                    editing.set(None);
                }
                Err(problem) => error.set(Some(problem.to_string())),
                _ => error.set(Some(crate::i18n::tr("ตรวจรายการไม่สำเร็จ"))),
            }
            saving.set(false);
            store.busy.set(false);
        });
    };
    rsx! {
        article { class: if checked { "review-card is-reviewed" } else { "review-card" }, "data-review-card": "{index}",
            div { class: "review-card-heading",
                span { class: "review-number", "{index + 1}" }
                div { class: "review-card-title", span { class: "field-hint", "{crate::i18n::tr(kind)}" } h3 { "{title}" } }
                strong { class: "review-amount", if draft.input.amount.is_empty() { {crate::i18n::tr("ยังไม่ทราบยอด")} } else { "{crate::i18n::currency_prefix()}{draft.input.amount}" } }
            }
            dl { class: "review-facts",
                div { dt { {crate::i18n::tr("วันที่")} } dd { "{draft.input.date}" } }
                div { dt { {crate::i18n::tr("บัญชีที่ใช้รับหรือจ่าย")} } dd { {draft.input.account.map(|id| account_label(&view, id)).unwrap_or_else(|| crate::i18n::tr("เลือกบัญชี"))} } }
                if draft.input.kind == TransactionKind::Transfer {
                    div { dt { {crate::i18n::tr("บัญชีปลายทาง")} } dd { {draft.input.destination.map(|id| account_label(&view, id)).unwrap_or_else(|| crate::i18n::tr("เลือกบัญชีปลายทาง"))} } }
                } else {
                    div { dt { {crate::i18n::tr("หมวดหมู่")} } dd { {draft.input.category.map(|c| crate::i18n::tr(c.label())).unwrap_or_else(|| crate::i18n::tr("เลือกหมวดหมู่"))} } }
                }
            }
            if let Some(link) = &draft.input.recurring {
                p { class: "field-hint", {view.recurring.iter().find(|plan| plan.id() == link.recurring).map(|plan| format!("{} · {}", plan.name().as_str(), link.month)).unwrap_or_else(|| link.month.clone())} }
            }
            if let Some(tax) = &draft.input.income_tax { p { class: "field-hint", {crate::i18n::text("ภาษี: ม.40({0}) · ยอดก่อนหัก {1} · หัก ณ ที่จ่าย {2} · VAT {3} · หักอื่น {4}", &[tax.section.map(|section| section.number().to_string()).unwrap_or_else(|| "—".into()), tax.gross.clone(), tax.withholding.clone(), tax.vat.clone(), tax.other_deductions.clone()])} } }
            if let Some(receipt) = &draft.input.receipt {
                details { class: "review-receipt", summary { {crate::i18n::tr("รายละเอียดใบเสร็จ")} }
                    for line in &receipt.lines { p { "{line.description} · {line.amount}" } }
                }
            }
            if let Some(input) = editing.read().clone() {
                div { class: "review-editor",
                    BatchEntryFields { index, input, view: view.clone(), editing }
                    if let Some(message) = error() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&message)}" } }
                    div { class: "review-card-actions",
                        button { id: "batch-save-{index}", class: "soft-button", disabled: *store.busy.read(), onclick: save, {crate::i18n::tr("เก็บการแก้ไขในบัตร")} }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| { editing.set(None); error.set(None); }, {crate::i18n::tr("ยกเลิก")} }
                    }
                }
            } else {
                if !draft.guidance.is_empty() { p { class: "field-hint review-guidance", "{crate::i18n::tr(&draft.guidance)}" } }
                div { class: "review-card-actions",
                    label { class: "review-check", r#for: "batch-reviewed-{index}",
                        input { id: "batch-reviewed-{index}", r#type: "checkbox", checked, disabled: *store.busy.read() || !missing.is_empty(), onchange: move |event| {
                            if event.checked() { reviewed.write().insert(index, draft.input.clone()); }
                            else { reviewed.write().remove(&index); }
                        } }
                        {crate::i18n::tr("ตรวจรายการนี้แล้ว")}
                    }
                    button { id: "batch-edit-{index}", class: "text-button", disabled: *store.busy.read(), onclick: move |_| {
                        reviewed.write().remove(&index);
                        store.batch_prepared.set(None);
                        if let Some((_, drafts)) = store.batch.peek().as_ref() && let Some(draft) = drafts.get(index) { editing.set(Some(draft.input.clone())); }
                    }, Icon { name: "edit", size: 18 } {crate::i18n::tr("แก้ไข")} }
                }
            }
        }
    }
}

#[component]
fn BatchEntryFields(
    index: usize,
    input: EntryInput,
    view: Dashboard,
    editing: Signal<Option<EntryInput>>,
) -> Element {
    let store = use_context::<UiState>();
    let categories = if input.kind == TransactionKind::Income {
        &Category::INCOME[..]
    } else {
        &Category::EXPENSE[..]
    };
    rsx! { fieldset { class: "review-fields", disabled: *store.busy.read(),
                        label { r#for: "batch-kind-{index}", {crate::i18n::text("ชนิดรายการ", &[])} }
                        select { id: "batch-kind-{index}", value: match input.kind { TransactionKind::Expense => "expense", TransactionKind::Income => "income", TransactionKind::Transfer => "transfer" },
                            onchange: move |event| edit_input(editing, |input| { input.kind = match event.value().as_str() { "income" => TransactionKind::Income, "transfer" => TransactionKind::Transfer, _ => TransactionKind::Expense }; input.category = None; input.destination = None; input.recurring = None; input.income_tax = None; }),
                            option { value: "expense", selected: input.kind == TransactionKind::Expense, {crate::i18n::text("รายจ่าย", &[])} } option { value: "income", selected: input.kind == TransactionKind::Income, {crate::i18n::text("รายรับ", &[])} } option { value: "transfer", disabled: input.receipt.is_some(), selected: input.kind == TransactionKind::Transfer, {crate::i18n::text("โอนเงิน", &[])} }
                        }
                        if input.kind == TransactionKind::Expense {
                            crate::recurring_picker::RecurringPicker { id: "batch-recurring-{index}", input: input.clone(), view: view.clone(), onchange: move |updated| edit_input(editing, |input| *input = updated) }
                        }
                        label { r#for: "batch-amount-{index}", {crate::i18n::text("ยอดเงิน (บาท)", &[])} }
                        input { id: "batch-amount-{index}", inputmode: "decimal", value: input.amount.clone(), placeholder: crate::i18n::text("ระบุยอดเงิน", &[]), oninput: move |event| edit_input(editing, |input| input.amount = event.value()) }
                        label { r#for: "batch-account-{index}", {crate::i18n::text("บัญชีที่ใช้รับหรือจ่าย", &[])} }
                        select { id: "batch-account-{index}", value: input.account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |event| edit_input(editing, |input| input.account = event.value().parse().ok()),
                            option { value: "", selected: input.account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                            for account in &view.accounts { if account.account.accepts_cash_entries() { option { value: "{account.account.id()}", selected: input.account == Some(account.account.id()), "{account.account.name().as_str()}" } } }
                        }
                        if input.kind == TransactionKind::Transfer {
                            label { r#for: "batch-destination-{index}", {crate::i18n::text("บัญชีปลายทาง", &[])} }
                            select { id: "batch-destination-{index}", value: input.destination.map(|id| id.to_string()).unwrap_or_default(), onchange: move |event| edit_input(editing, |input| input.destination = event.value().parse().ok()),
                                option { value: "", selected: input.destination.is_none(), {crate::i18n::text("เลือกบัญชีปลายทาง", &[])} }
                                for account in &view.accounts { if account.account.accepts_cash_entries() && Some(account.account.id()) != input.account { option { value: "{account.account.id()}", selected: input.destination == Some(account.account.id()), "{account.account.name().as_str()}" } } }
                            }
                        } else {
                            label { r#for: "batch-category-{index}", {crate::i18n::text("หมวดหมู่", &[])} }
                            select { id: "batch-category-{index}", value: input.category.map(|c| c.code()).unwrap_or(""), onchange: move |event| edit_input(editing, |input| input.category = Category::from_code(&event.value()).ok()),
                                option { value: "", selected: input.category.is_none(), {crate::i18n::text("เลือกหมวดหมู่", &[])} }
                                for category in categories { option { value: category.code(), selected: input.category == Some(*category), "{crate::i18n::tr(category.label())}" } }
                            }
                        }
                        if input.kind == TransactionKind::Income && view.thai_tax_enabled && view.currency == Currency::Thb {
                            crate::income_tax::IncomeTaxFields { input: input.clone(), id: "batch-tax-{index}", onchange: move |updated| edit_input(editing, |input| *input = updated) }
                        }
                        label { r#for: "batch-date-{index}", {crate::i18n::text("วันที่", &[])} }
                        input { id: "batch-date-{index}", r#type: "date", min: "1900-01-01", max: "{view.today}", value: input.date.clone(), onchange: move |event| edit_input(editing, |input| input.date = event.value()) }
                        if let Some(receipt) = input.receipt.clone() {
                            crate::receipt_editor::ReceiptEditor { receipt, total: input.amount.clone(), id: "batch-receipt-{index}", onchange: move |receipt| edit_input(editing, |input| input.receipt = Some(receipt)) }
                        }
                        label { r#for: "batch-note-{index}", {crate::i18n::text("รายละเอียด", &[])} }
                        textarea { id: "batch-note-{index}", rows: "3", maxlength: Note::MAX_CHARS, value: input.note.clone(), oninput: move |event| edit_input(editing, |input| input.note = event.value()) }
    } }
}
