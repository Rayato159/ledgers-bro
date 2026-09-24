use crate::{HostInfo, artwork::*, components::*, receipt::ReceiptScanner, state::*};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

#[component]
pub fn QuickEntryPage(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let host = use_context::<HostInfo>();
    let mut text = use_signal(String::new);
    let mut previous_text = use_signal(String::new);
    let capturing = use_signal(|| false);
    // Chips and voice input also update the composer. They must invalidate the
    // previous confirmation just like typing, even without a DOM input event.
    use_effect(move || {
        let current = text.read().clone();
        if *previous_text.peek() != current {
            previous_text.set(current);
            store.batch.set(None);
            store.batch_prepared.set(None);
            store.model_choices.set(None);
            store.prepared.set(None);
            store.input.set(None);
            store.receipt.set(None);
            store.guidance.set(String::new());
        }
    });
    use_drop(move || store.cancel_model());
    let first_account = view
        .accounts
        .iter()
        .find(|a| !a.account.is_archived())
        .map(|a| format!("\"{}\"", a.account.name().as_str()))
        .unwrap_or_else(|| "\"ชื่อบัญชี\"".into());
    let expense_sample = format!("จ่าย 80 จาก {first_account} หมวด อาหาร");
    let income_sample = format!("รับ 30000 เข้า {first_account} หมวด เงินเดือน");
    let input = store.input.read().clone();
    let prepared = store.prepared.read().clone();
    rsx! {
        section { class: "page-heading", div { h1 { "บันทึกด่วน" } p { class: "muted", "พิมพ์ พูด หรือสแกนใบเสร็จ แล้วตรวจให้ตรงก่อนบันทึก" } } }
        button { class: "manual-entry-shortcut", disabled: *capturing.read() || (*store.busy.read() && store.model_operation.read().is_none()), onclick: move |_| store.new_entry(),
            Icon { name: "edit", size: 24 }
            div { strong { "กรอกเอง" } span { "เลือกยอด บัญชี และหมวดได้เอง ไม่ต้องใช้ AI" } }
            Icon { name: "arrow", size: 20 }
        }
        div { class: "entry-layout",
            section { class: "card chat-card",
                div { class: "chat-greeting illustrated-greeting", img { class: "phone-mascot", src: host.art.phone.clone(), alt: "ทานูกิถือโทรศัพท์พร้อมจดรายการ" } div { strong { "จดไว้ เดี๋ยวช่วยจัดให้" } p { "เล่าเรื่องเงินวันนี้ให้ฟัง\nหรือหยิบใบเสร็จมาให้ช่วยอ่าน" } } }
                p { class: "field-hint", "กดตัวอย่างเพื่อเติมข้อความ แล้วแก้เป็นรายการของเรา" }
                fieldset { class: "prompt-chips input-methods", disabled: *capturing.read() || *store.busy.read(),
                    button { class: "sample-chip", onclick: move |_| text.set("กาแฟ 80".into()), "กาแฟ 80" }
                    button { class: "sample-chip", onclick: move |_| text.set(expense_sample.clone()), "จ่ายค่าอาหาร" }
                    button { class: "sample-chip", onclick: move |_| text.set(income_sample.clone()), "รับเงินเดือน" }
                    button { class: "sample-chip", onclick: move |_| text.set("ซื้อไก่ทอดไป 30 บาท และได้เงินจาก Facebook 400 บาท บันทึกลงเงินสด และ กรุงไทยตามลำดับ".into()), "หลายรายการ" }
                    button { class: "sample-chip", onclick: move |_| text.set("สรุป เดือนนี้".into()), "สรุปเดือนนี้" }
                }
                form { class: "chat-compose", onsubmit: move |event| { event.prevent_default(); if !*capturing.peek() { store.resolve_text(text()); } },
                    label { r#for: "quick-text", class: "sr-only", "พิมพ์รายการแบบด่วน" }
                    textarea { id: "quick-text", placeholder: "เช่น เมื่อวานซื้อกาแฟ 80 บาท จ่ายเงินสด", rows: "3", maxlength: 1000, required: true, value: "{text}", disabled: *store.busy.read() || *capturing.read(), oninput: move |event| { text.set(event.value()); store.batch.set(None); store.batch_prepared.set(None); store.model_choices.set(None); store.prepared.set(None); store.input.set(None); store.receipt.set(None); store.guidance.set(String::new()); } }
                    crate::voice::VoiceInput { text, capturing }
                    button { class: "primary", r#type: "submit", disabled: *capturing.read() || *store.busy.read() || text.read().trim().is_empty(), "อ่านรายการ" Icon { name: "arrow", size: 17 } }
                }
                if store.model_operation.read().is_some() { crate::model::ModelProgress {} }
                else if !store.guidance.read().is_empty() { div { class: "assistant-message", role: "status", "{store.guidance}" } }
                fieldset { class: "input-methods receipt-method", disabled: *capturing.read(), ReceiptScanner {} }
                details { class: "model-disclosure", summary { Icon { name: "chat", size: 19 } "AI ในเครื่อง" } crate::model::ModelSettings { capturing } }
                div { class: "chat-help", strong { "ตัวอย่างรูปแบบที่รองรับ" } code { "จ่าย 80 จาก เงินสด หมวด อาหาร" } code { "โอน 1000 จาก ธนาคาร ไป เงินสด" } code { "… วันที่ เมื่อวาน โน้ต \"ข้าวกลางวัน\"" } p { "การจ่ายหนี้บัตรใช้โอนไปบัญชีบัตรเครดิต ดอกเบี้ยหรือค่าธรรมเนียมให้บันทึกเป็นรายจ่ายแยก" } }
            }
            section { class: "card entry-editor",
                if view.accounts.is_empty() {
                    EmptyState { title: "เพิ่มบัญชีก่อนเริ่มจด", body: "แอปต้องรู้ว่าเงินเข้าหรือออกจากบัญชีไหน จะได้คำนวณยอดถูก" }
                    button { class: "primary", onclick: move |_| store.account_form.set(true), "เพิ่มบัญชีแรก" }
                } else if let Some((source, drafts)) = store.batch.read().clone() {
                    crate::batch_entry::BatchReview { source, drafts, view: view.clone() }
                } else if let Some(prepared) = prepared {
                    Confirmation { prepared, view: view.clone() }
                } else if let Some((source, drafts)) = store.model_choices.read().clone() {
                    crate::model::ModelChoices { source, drafts, view: view.clone() }
                } else if let Some(input) = input {
                    EntryForm { input, view: view.clone() }
                } else {
                    div { class: "draft-empty", div { class: "draft-illustration", ArtIcon { name: "receipt", size: 88 } } h2 { "รายการรอตรวจจะอยู่ตรงนี้" } p { "เลือกใบเสร็จหรือพิมพ์รายการทางซ้าย\nยังไม่มีอะไรถูกบันทึก จนกว่าจะยืนยัน" }
                        div { class: "category-art-preview", for category in [Category::Food, Category::Snacks, Category::Salary] { span { title: category.label(), ArtIcon { name: category_art(category), size: 44 } } } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ManualEntryPage(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let input = store.input.read().clone();
    let prepared = store.prepared.read().clone();
    rsx! {
        section { class: "page-heading",
            div { h1 { "บันทึกรายการเอง" } p { class: "muted", "กรอกข้อมูล แล้วตรวจให้ตรงก่อนบันทึก" } }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.page.set(Page::Chat), Icon { name: "chat", size: 20 } "ใช้บันทึกด่วน" }
        }
        section { class: "card entry-editor manual-entry-editor",
            if view.accounts.is_empty() {
                EmptyState { title: "เพิ่มบัญชีก่อนเริ่มจด", body: "เลือกบัญชีที่จะใช้รับหรือจ่ายเงินก่อน" }
                button { class: "primary", onclick: move |_| store.account_form.set(true), "เพิ่มบัญชีแรก" }
            } else if let Some(prepared) = prepared {
                Confirmation { prepared, view: view.clone() }
            } else if let Some(input) = input {
                EntryForm { input, view: view.clone() }
            } else {
                EmptyState { title: "พร้อมจดรายการถัดไป", body: "เพิ่มรายรับ รายจ่าย หรือโอนเงินได้ด้วยตัวเอง" }
                button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.new_entry(), Icon { name: "plus", size: 20 } "เพิ่มรายการใหม่" }
            }
        }
    }
}

#[component]
fn EntryForm(input: EntryInput, view: Dashboard) -> Element {
    let store = use_context::<UiState>();
    let missing = missing_entry_fields(&input);
    let can_preview = missing.is_empty();
    let kind = input.kind;
    let has_receipt = input.receipt.is_some();
    let categories: &[Category] = if kind == TransactionKind::Income {
        &Category::INCOME
    } else {
        &Category::EXPENSE
    };
    let account_value = input.account.map(|id| id.to_string()).unwrap_or_default();
    let destination_value = input
        .destination
        .map(|id| id.to_string())
        .unwrap_or_default();
    rsx! {
        div { class: "section-heading", h2 { "รายละเอียดรายการ" } span { class: "status-pill", "ยังไม่บันทึก" } }
        div { class: "kind-tabs", role: "group", "aria-label": "ชนิดรายการ",
            for (target, label) in [(TransactionKind::Expense, "รายจ่าย"), (TransactionKind::Income, "รายรับ"), (TransactionKind::Transfer, "โอนเงิน")] {
                button { class: if target == kind { "selected" } else { "" }, "aria-pressed": target == kind, disabled: *store.busy.read() || (has_receipt && target == TransactionKind::Transfer), onclick: move |_| store.update_entry(|i| { i.kind = target; i.category = None; i.destination = None; }), "{label}" }
            }
        }
        form { onsubmit: move |event| { event.prevent_default(); let input = store.input.read().clone(); if let Some(input) = input { store.send(Command::Preview(input)); } },
            label { r#for: "amount", "จำนวนเงิน (บาท)" } input { id: "amount", class: "amount-input", inputmode: "decimal", required: true, maxlength: 18, placeholder: "0.00", value: input.amount.clone(), disabled: *store.busy.read(), oninput: move |event| store.update_entry(|i| i.amount = event.value()) }
            label { r#for: "entry-account", if kind == TransactionKind::Income { "เงินเข้าบัญชี" } else { "จากบัญชี" } }
            select { id: "entry-account", required: true, value: account_value, disabled: *store.busy.read(), onchange: move |event| store.update_entry(|i| i.account = event.value().parse().ok()),
                option { value: "", disabled: true, "เลือกบัญชี" }
                for item in &view.accounts { if !item.account.is_archived() { option { value: "{item.account.id()}", "{item.account.name().as_str()} · {item.account.kind().label()}" } } }
            }
            if kind == TransactionKind::Transfer {
                label { r#for: "destination", "ไปบัญชี" }
                select { id: "destination", required: true, value: destination_value, disabled: *store.busy.read(), onchange: move |event| store.update_entry(|i| i.destination = event.value().parse().ok()),
                    option { value: "", disabled: true, "เลือกบัญชีปลายทาง" }
                    for item in &view.accounts { if !item.account.is_archived() && Some(item.account.id()) != input.account { option { value: "{item.account.id()}", "{item.account.name().as_str()}" } } }
                }
                p { class: "field-hint", "การโอนและการจ่ายยอดหนี้บัตรไม่เพิ่มรายรับรายจ่าย" }
            } else {
                fieldset { class: "category-field", legend { "หมวดหมู่" }
                    div { class: "category-grid", for category in categories { { let category = *category; rsx! {
                        button { r#type: "button", class: if input.category == Some(category) { "category-tile selected" } else { "category-tile" },
                            "aria-pressed": input.category == Some(category), disabled: *store.busy.read(), onclick: move |_| store.update_entry(|i| i.category = Some(category)),
                            ArtIcon { name: category_art(category), size: 42 } span { "{category.label()}" }
                        }
                    } } } }
                }
            }
            label { r#for: "entry-date", "วันที่รายการ" } input { id: "entry-date", r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: input.date, disabled: *store.busy.read(), onchange: move |event| store.update_entry(|i| i.date = event.value()) }
            if let Some(receipt) = input.receipt.clone() { crate::receipt_editor::ReceiptEditor { receipt, total: input.amount.clone() } }
            label { r#for: "entry-note", "รายละเอียดเพิ่มเติม (ไม่จำเป็น)" } textarea { id: "entry-note", maxlength: 500, rows: "3", placeholder: "เช่น ข้าวกลางวัน", value: input.note, disabled: *store.busy.read(), oninput: move |event| store.update_entry(|i| i.note = event.value()) }
            if !can_preview { p { class: "batch-question", role: "status", "ยังขาด: {missing.join(\" · \")} — เติมข้อมูลเหล่านี้ก่อนบันทึก" } }
            button { class: "primary full-width", r#type: "submit", disabled: *store.busy.read() || !can_preview, "ตรวจรายการก่อนบันทึก" Icon { name: "arrow", size: 18 } }
        }
    }
}

#[component]
fn Confirmation(prepared: PreparedEntry, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let Some((label, amount, account)) = entry_label(&prepared.entry) else {
        return rsx! { p { role: "alert", "ไม่สามารถแสดงรายการนี้ได้" } };
    };
    let account = account_label(&view, account);
    let destination = match prepared.entry.kind() {
        EntryKind::Transfer { to, .. } => Some(account_label(&view, *to)),
        _ => None,
    };
    let salary = matches!(
        prepared.entry.kind(),
        EntryKind::Income {
            category: Category::Salary,
            ..
        }
    );
    let prepared_for_save = prepared.clone();
    rsx! {
        div { class: "confirmation",
            span { class: "confirm-symbol", Icon { name: "check", size: 30 } } h2 { "ตรงกับรายการของเราไหม?" }
            strong { class: "confirm-amount", "฿{money_label(amount)}" }
            dl { div { dt { "รายการ" } dd { "{label}" } } div { dt { "บัญชี" } dd { "{account}" } } if let Some(destination) = destination { div { dt { "ปลายทาง" } dd { "{destination}" } } } div { dt { "วันที่" } dd { "{prepared.entry.date()}" } } if !prepared.entry.note().as_str().is_empty() { div { class: "confirmation-note", dt { "รายละเอียด" } dd { "{prepared.entry.note().as_str()}" } } } }
            if salary { p { class: "inline-warning", "นี่คือยอดเงินที่รับเข้าบัญชี ยังไม่ใช้แทนเงินเดือนก่อนหักสำหรับคำนวณภาษี" } }
            p { class: "field-hint", "ยังไม่ได้บันทึก กดยืนยันเมื่อรายละเอียดถูกต้อง" }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.send(Command::Commit(prepared_for_save.clone())), if *store.busy.read() { "กำลังบันทึก…" } else { "ยืนยันบันทึก" } }
            button { class: "text-button full-width", disabled: *store.busy.read(), onclick: move |_| store.prepared.set(None), "กลับไปแก้ไข" }
        }
    }
}
