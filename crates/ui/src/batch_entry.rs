use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

fn edit(mut store: UiState, index: usize, apply: impl FnOnce(&mut EntryInput)) {
    store.batch_prepared.set(None);
    store.notice.set(None);
    if let Some((_, drafts)) = store.batch.write().as_mut()
        && let Some(draft) = drafts.get_mut(index)
    {
        let amount = draft.input.amount.clone();
        apply(&mut draft.input);
        if amount != draft.input.amount
            && let Some(receipt) = &mut draft.input.receipt
        {
            receipt.reviewed = false;
        }
        if draft.input.account.is_some() {
            draft.guidance.clear();
        }
    }
}

#[component]
pub(crate) fn BatchReview(source: String, drafts: Vec<ModelDraft>, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let prepared = store.batch_prepared.read().clone();
    let complete = !drafts.is_empty()
        && drafts
            .iter()
            .all(|draft| missing_entry_fields(&draft.input).is_empty());
    let missing_summary = drafts
        .iter()
        .enumerate()
        .filter_map(|(index, draft)| {
            let fields = missing_entry_fields(&draft.input);
            (!fields.is_empty()).then(|| {
                crate::i18n::text(
                    "รายการที่ {0}: {1}",
                    &[(index + 1).to_string(), crate::i18n::field_list(&fields)],
                )
            })
        })
        .collect::<Vec<_>>()
        .join(" / ");
    let inputs: Vec<_> = drafts.iter().map(|draft| draft.input.clone()).collect();
    rsx! {
        h2 { {crate::i18n::text("ตรวจ {0} รายการก่อนบันทึก", &[format!("{}", drafts.len())])} }
        details { class: "batch-source", summary { {crate::i18n::text("ดูข้อความต้นฉบับ", &[])} } pre { class: "model-source", "{source}" } }
        if let Some(prepared) = prepared {
            div { class: "assistant-message", role: "status", {crate::i18n::text("ข้อมูลครบแล้ว ตรวจทุกยอดและบัญชีอีกครั้ง จากนั้นยืนยันบันทึกทั้งชุด", &[])} }
            for (index, item) in prepared.iter().enumerate() {
                if let Some((label, amount, account)) = entry_label(&item.entry) {
                    div { class: "batch-item",
                        h3 { "{index + 1}. {crate::i18n::tr(&label)} {crate::i18n::currency_prefix()}{money_label(amount)}" }
                        p { {crate::i18n::text("บัญชี {0} · {1}", &[account_label(&view, account).to_string(), format!("{}", item.entry.date())])} }
                        if let EntryKind::Transfer { to, .. } = item.entry.kind() { p { {crate::i18n::text("ไป {0}", &[account_label(&view, *to).to_string()])} } }
                        if let EntryKind::Expense { category, .. } | EntryKind::Income { category, .. } = item.entry.kind() { p { {crate::i18n::text("หมวด {0}", &[crate::i18n::tr(category.label()).to_string()])} } }
                        p { "{item.entry.note().as_str()}" }
                        if let Some(link) = &item.recurring { p { class: "batch-question", {crate::i18n::text("ผูกกับ {0} · {1}", &[link.schedule.name().as_str().to_string(), crate::recurring_picker::installment_label(&link.schedule, link.month).to_string()])} } }
                    }
                }
            }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.send(Command::CommitBatch(prepared.clone())), {crate::i18n::text("ยืนยันบันทึกทั้งหมด", &[])} }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.batch_prepared.set(None), {crate::i18n::text("กลับไปแก้รายละเอียด", &[])} }
        } else {
            for (index, draft) in drafts.iter().enumerate() {
                { let input = &draft.input;
                  let missing = missing_entry_fields(input);
                  let label = match input.kind { TransactionKind::Income => "รายรับ", TransactionKind::Expense => "รายจ่าย", TransactionKind::Transfer => "โอนเงิน" };
                  let categories = if input.kind == TransactionKind::Income { &Category::INCOME[..] } else { &Category::EXPENSE[..] };
                  rsx! {
                    fieldset { class: "batch-item", disabled: *store.busy.read(),
                        legend { {crate::i18n::text("รายการที่ {0} · {1}", &[format!("{}", index + 1), label.to_string()])} }
                        if !missing.is_empty() { p { class: "batch-question", role: "status", {crate::i18n::text("ยังขาด: {0} — เติมช่องด้านล่างให้ครบก่อนบันทึก", &[crate::i18n::field_list(&missing)])} } }
                        if !draft.guidance.is_empty() { p { class: "field-hint", "{draft.guidance}" } }
                        label { r#for: "batch-kind-{index}", {crate::i18n::text("ชนิดรายการ", &[])} }
                        select { id: "batch-kind-{index}", value: match input.kind { TransactionKind::Expense => "expense", TransactionKind::Income => "income", TransactionKind::Transfer => "transfer" },
                            onchange: move |event| edit(store, index, |input| { input.kind = match event.value().as_str() { "income" => TransactionKind::Income, "transfer" => TransactionKind::Transfer, _ => TransactionKind::Expense }; input.category = None; input.destination = None; input.recurring = None; input.income_tax = None; }),
                            option { value: "expense", selected: input.kind == TransactionKind::Expense, {crate::i18n::text("รายจ่าย", &[])} } option { value: "income", selected: input.kind == TransactionKind::Income, {crate::i18n::text("รายรับ", &[])} } option { value: "transfer", disabled: input.receipt.is_some(), selected: input.kind == TransactionKind::Transfer, {crate::i18n::text("โอนเงิน", &[])} }
                        }
                        if input.kind == TransactionKind::Expense {
                            crate::recurring_picker::RecurringPicker { id: "batch-recurring-{index}", input: input.clone(), view: view.clone(), onchange: move |updated| edit(store, index, |input| *input = updated) }
                        }
                        label { r#for: "batch-amount-{index}", {crate::i18n::text("ยอดเงิน (บาท)", &[])} }
                        input { id: "batch-amount-{index}", inputmode: "decimal", value: input.amount.clone(), placeholder: crate::i18n::text("ระบุยอดเงิน", &[]), oninput: move |event| edit(store, index, |input| input.amount = event.value()) }
                        label { r#for: "batch-account-{index}", {crate::i18n::text("บัญชีที่ใช้รับหรือจ่าย", &[])} }
                        select { id: "batch-account-{index}", value: input.account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |event| edit(store, index, |input| input.account = event.value().parse().ok()),
                            option { value: "", selected: input.account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                            for account in &view.accounts { if account.account.accepts_cash_entries() { option { value: "{account.account.id()}", selected: input.account == Some(account.account.id()), "{account.account.name().as_str()}" } } }
                        }
                        if input.kind == TransactionKind::Transfer {
                            label { r#for: "batch-destination-{index}", {crate::i18n::text("บัญชีปลายทาง", &[])} }
                            select { id: "batch-destination-{index}", value: input.destination.map(|id| id.to_string()).unwrap_or_default(), onchange: move |event| edit(store, index, |input| input.destination = event.value().parse().ok()),
                                option { value: "", selected: input.destination.is_none(), {crate::i18n::text("เลือกบัญชีปลายทาง", &[])} }
                                for account in &view.accounts { if account.account.accepts_cash_entries() && Some(account.account.id()) != input.account { option { value: "{account.account.id()}", selected: input.destination == Some(account.account.id()), "{account.account.name().as_str()}" } } }
                            }
                        } else {
                            label { r#for: "batch-category-{index}", {crate::i18n::text("หมวดหมู่", &[])} }
                            select { id: "batch-category-{index}", value: input.category.map(|c| c.code()).unwrap_or(""), onchange: move |event| edit(store, index, |input| input.category = Category::from_code(&event.value()).ok()),
                                option { value: "", selected: input.category.is_none(), {crate::i18n::text("เลือกหมวดหมู่", &[])} }
                                for category in categories { option { value: category.code(), selected: input.category == Some(*category), "{crate::i18n::tr(category.label())}" } }
                            }
                        }
                        if input.kind == TransactionKind::Income && view.thai_tax_enabled && view.currency == Currency::Thb {
                            crate::income_tax::IncomeTaxFields { input: input.clone(), id: "batch-tax-{index}", onchange: move |updated| edit(store, index, |input| *input = updated) }
                        }
                        label { r#for: "batch-date-{index}", {crate::i18n::text("วันที่", &[])} }
                        input { id: "batch-date-{index}", r#type: "date", min: "1900-01-01", max: "{view.today}", value: input.date.clone(), onchange: move |event| edit(store, index, |input| input.date = event.value()) }
                        if let Some(receipt) = input.receipt.clone() {
                            crate::receipt_editor::ReceiptEditor { receipt, total: input.amount.clone(), id: "batch-receipt-{index}", onchange: move |receipt| edit(store, index, |input| input.receipt = Some(receipt)) }
                        }
                        label { r#for: "batch-note-{index}", {crate::i18n::text("รายละเอียด", &[])} }
                        input { id: "batch-note-{index}", value: input.note.clone(), oninput: move |event| edit(store, index, |input| input.note = event.value()) }
                    }
                  }
                }
            }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.account_form.set(true), {crate::i18n::text("เพิ่มบัญชีที่ยังไม่มี", &[])} }
            if !complete { p { class: "batch-question", role: "status", {crate::i18n::text("ยังบันทึกไม่ได้ — {0} · เลือกช่องที่ยังว่างในแต่ละรายการด้านบน", std::slice::from_ref(&missing_summary))} } }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| {
                if complete { store.send(Command::PreviewBatch(inputs.clone())); }
                else { store.notice.set(Some((true, format!("กรุณาเติมข้อมูลก่อนบันทึก — {missing_summary}")))); }
            }, if complete { {crate::i18n::text("ตรวจรายการทั้งหมดก่อนบันทึก", &[])} } else { {crate::i18n::text("ตรวจข้อมูลที่ยังขาด", &[])} } }
        }
    }
}
