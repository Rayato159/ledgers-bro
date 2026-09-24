use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

fn edit(mut store: UiState, index: usize, apply: impl FnOnce(&mut EntryInput)) {
    store.batch_prepared.set(None);
    if let Some((_, drafts)) = store.batch.write().as_mut()
        && let Some(draft) = drafts.get_mut(index)
    {
        apply(&mut draft.input);
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
    let inputs: Vec<_> = drafts.iter().map(|draft| draft.input.clone()).collect();
    rsx! {
        h2 { "ตรวจ {drafts.len()} รายการจากข้อความ" }
        p { class: "model-source", "{source}" }
        if let Some(prepared) = prepared {
            div { class: "assistant-message", role: "status", "ข้อมูลครบแล้ว ตรวจทุกยอดและบัญชีอีกครั้ง จากนั้นยืนยันบันทึกทั้งชุด" }
            for (index, item) in prepared.iter().enumerate() {
                if let Some((label, amount, account)) = entry_label(&item.entry) {
                    div { class: "batch-item",
                        h3 { "{index + 1}. {label} ฿{money_label(amount)}" }
                        p { "บัญชี {account_label(&view, account)} · {item.entry.date()}" }
                        if let EntryKind::Transfer { to, .. } = item.entry.kind() { p { "ไป {account_label(&view, *to)}" } }
                        if let EntryKind::Expense { category, .. } | EntryKind::Income { category, .. } = item.entry.kind() { p { "หมวด {category.label()}" } }
                        p { "{item.entry.note().as_str()}" }
                    }
                }
            }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.send(Command::CommitBatch(prepared.clone())), "ยืนยันบันทึกทั้งหมด" }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.batch_prepared.set(None), "กลับไปแก้รายละเอียด" }
        } else {
            for (index, draft) in drafts.iter().enumerate() {
                { let input = &draft.input;
                  let missing = missing_entry_fields(input);
                  let label = match input.kind { TransactionKind::Income => "รายรับ", TransactionKind::Expense => "รายจ่าย", TransactionKind::Transfer => "โอนเงิน" };
                  let categories = if input.kind == TransactionKind::Income { &Category::INCOME[..] } else { &Category::EXPENSE[..] };
                  rsx! {
                    fieldset { class: "batch-item", disabled: *store.busy.read(),
                        legend { "รายการที่ {index + 1} · {label}" }
                        if !missing.is_empty() { p { class: "batch-question", role: "status", "ยังขาด: {missing.join(\" · \")} — เติมช่องด้านล่างให้ครบก่อนบันทึก" } }
                        if !draft.guidance.is_empty() { p { class: "field-hint", "{draft.guidance}" } }
                        label { r#for: "batch-kind-{index}", "ชนิดรายการ" }
                        select { id: "batch-kind-{index}", value: match input.kind { TransactionKind::Expense => "expense", TransactionKind::Income => "income", TransactionKind::Transfer => "transfer" },
                            onchange: move |event| edit(store, index, |input| { input.kind = match event.value().as_str() { "income" => TransactionKind::Income, "transfer" => TransactionKind::Transfer, _ => TransactionKind::Expense }; input.category = None; input.destination = None; }),
                            option { value: "expense", "รายจ่าย" } option { value: "income", "รายรับ" } option { value: "transfer", "โอนเงิน" }
                        }
                        label { r#for: "batch-amount-{index}", "ยอดเงิน (บาท)" }
                        input { id: "batch-amount-{index}", inputmode: "decimal", value: input.amount.clone(), placeholder: "ระบุยอดเงิน", oninput: move |event| edit(store, index, |input| input.amount = event.value()) }
                        label { r#for: "batch-account-{index}", "บัญชีที่ใช้รับหรือจ่าย" }
                        select { id: "batch-account-{index}", value: input.account.map(|id| id.to_string()).unwrap_or_default(), onchange: move |event| edit(store, index, |input| input.account = event.value().parse().ok()),
                            option { value: "", selected: input.account.is_none(), "เลือกบัญชี" }
                            for account in &view.accounts { if !account.account.is_archived() { option { value: "{account.account.id()}", selected: input.account == Some(account.account.id()), "{account.account.name().as_str()}" } } }
                        }
                        if input.kind == TransactionKind::Transfer {
                            label { r#for: "batch-destination-{index}", "บัญชีปลายทาง" }
                            select { id: "batch-destination-{index}", value: input.destination.map(|id| id.to_string()).unwrap_or_default(), onchange: move |event| edit(store, index, |input| input.destination = event.value().parse().ok()),
                                option { value: "", selected: input.destination.is_none(), "เลือกบัญชีปลายทาง" }
                                for account in &view.accounts { if !account.account.is_archived() && Some(account.account.id()) != input.account { option { value: "{account.account.id()}", selected: input.destination == Some(account.account.id()), "{account.account.name().as_str()}" } } }
                            }
                        } else {
                            label { r#for: "batch-category-{index}", "หมวดหมู่" }
                            select { id: "batch-category-{index}", value: input.category.map(|c| c.code()).unwrap_or(""), onchange: move |event| edit(store, index, |input| input.category = Category::from_code(&event.value()).ok()),
                                option { value: "", selected: input.category.is_none(), "เลือกหมวดหมู่" }
                                for category in categories { option { value: category.code(), selected: input.category == Some(*category), "{category.label()}" } }
                            }
                        }
                        label { r#for: "batch-date-{index}", "วันที่" }
                        input { id: "batch-date-{index}", r#type: "date", min: "1900-01-01", max: "{view.today}", value: input.date.clone(), onchange: move |event| edit(store, index, |input| input.date = event.value()) }
                        label { r#for: "batch-note-{index}", "รายละเอียด" }
                        input { id: "batch-note-{index}", value: input.note.clone(), oninput: move |event| edit(store, index, |input| input.note = event.value()) }
                    }
                  }
                }
            }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.account_form.set(true), "เพิ่มบัญชีที่ยังไม่มี" }
            button { class: "primary full-width", disabled: *store.busy.read() || !complete, onclick: move |_| store.send(Command::PreviewBatch(inputs.clone())), "ตรวจรายการทั้งหมดก่อนบันทึก" }
        }
    }
}
