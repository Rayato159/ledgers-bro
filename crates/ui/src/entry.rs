use crate::{
    HostInfo,
    artwork::*,
    components::*,
    receipt::{ReceiptAttachments, ReceiptDestination, ReceiptUpload},
    state::*,
};
use dioxus::html::HasFileData;
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

#[component]
pub fn QuickEntryPage(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let host = use_context::<HostInfo>();
    let mut text = store.composer;
    let mut previous_text = use_signal(|| text.peek().clone());
    let mut dragging = use_signal(|| false);
    let capturing = use_signal(|| false);
    let mut examples = use_signal(|| false);
    let mut reviewing = use_signal(|| false);
    let mut had_review = use_signal(|| false);
    use_effect(move || {
        let current = text.read().clone();
        if *previous_text.peek() != current {
            previous_text.set(current);
            store.prompt_drafts.set(None);
            store.prompt_prepared.set(None);
            store.batch.set(None);
            store.batch_prepared.set(None);
            store.model_choices.set(None);
            store.prepared.set(None);
            store.input.set(None);
            store.guidance.set(String::new());
        }
    });
    use_drop(move || store.cancel_model());
    use_effect(move || {
        let ready = store.prompt_drafts.read().is_some()
            || store.batch.read().is_some()
            || store.prepared.read().is_some()
            || store.model_choices.read().is_some()
            || store.input.read().is_some();
        if ready && !*had_review.peek() {
            reviewing.set(true);
        }
        if !ready {
            reviewing.set(false);
        }
        had_review.set(ready);
    });
    rsx! {
        section { class: "page-heading", h1 { {crate::i18n::tr("เพิ่มรายการ")} } }
        crate::navigation::EntryToolbar { manual: false }
        section { class: "chat-workspace",
            form { class: if dragging() { "chat-compose compact-compose receipt-drag-over" } else { "chat-compose compact-compose" },
                ondragover: move |e| { e.prevent_default(); if !*store.busy.peek() && !*capturing.peek() { dragging.set(true); } },
                ondragleave: move |_| dragging.set(false),
                ondrop: move |e| { e.prevent_default(); dragging.set(false); if !*capturing.peek() && host.receipt_ocr_available { store.scan_receipts(e.files(), ReceiptDestination::Prompt); } },
                onsubmit: move |e| { e.prevent_default(); if !*capturing.peek() { store.resolve_text(text()); } },
                label { r#for: "quick-text", class: "sr-only", {crate::i18n::tr("พิมพ์รายการแบบด่วน")} }
                textarea { id: "quick-text", placeholder: crate::i18n::tr("เช่น เมื่อวานซื้อกาแฟ 80 บาท จ่ายเงินสด"), rows: "5", maxlength: MAX_RECEIPT_PROMPT_CHARS, required: true, value: "{text}", disabled: *store.busy.read() || *capturing.read(), oninput: move |e| text.set(e.value()) }
                ReceiptAttachments {}
                div { class: "composer-tools",
                    ReceiptUpload { destination: ReceiptDestination::Prompt, disabled: *capturing.read(), compact: true }
                    crate::voice::VoiceInput { text, capturing, compact: true }
                    button { r#type: "button", class: "icon-button", title: crate::i18n::tr("ตัวอย่างคำสั่ง"), "aria-label": crate::i18n::tr("ตัวอย่างคำสั่ง"), "aria-haspopup": "dialog", disabled: *capturing.read() || *store.busy.read(), onclick: move |_| examples.set(true), Icon { name: "list", size: 20 } }
                    crate::model::ModelSettings { capturing, compact: true }
                    button { class: "primary composer-send", r#type: "submit", title: crate::i18n::tr("อ่านรายการ"), "aria-label": crate::i18n::tr("อ่านรายการ"), disabled: *capturing.read() || *store.busy.read() || text.read().trim().is_empty(), Icon { name: "arrow", size: 20 } }
                }
            }
            if store.model_operation.read().is_some() { crate::model::ModelProgress {} }
            else if !store.guidance.read().is_empty() { p { class: "assistant-message", role: "status", "{crate::i18n::tr(&store.guidance.read())}" } }
            if had_review() { button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| reviewing.set(true), Icon { name: "check", size: 18 } {crate::i18n::tr("ตรวจรายการ")} } }
        }
        if examples() {
            dialog { id: "prompt-examples-dialog", class: "account-dialog prompt-examples-dialog", "aria-labelledby": "prompt-examples-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('prompt-examples-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); examples.set(false); },
                div { class: "section-heading", h2 { id: "prompt-examples-title", {crate::i18n::tr("ตัวอย่างคำสั่ง")} } button { class: "icon-button", "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| examples.set(false), Icon { name: "close", size: 20 } } }
                p { class: "field-hint", {crate::i18n::tr("หลายคำสั่งใช้ขึ้นบรรทัดใหม่ ครั้งละไม่เกิน 8 คำสั่ง ชื่อที่มีคำว่า และ ให้ใส่เครื่องหมายคำพูด")} }
                div { class: "prompt-example-grid prompt-catalog-grid", for kind in PromptKind::ALL {
                    { let (icon, tone) = crate::prompt_examples::example_art(kind); rsx! {
                        crate::prompt_examples::PromptExample { title: crate::i18n::tr(kind.label()), sample: kind.example(), icon, tone, compact: true,
                            selected: *text.read() == crate::prompt_examples::currency_example(kind.example()), disabled: *capturing.read() || *store.busy.read(),
                            onselect: move |sample| { text.set(sample); examples.set(false); },
                        }
                    } }
                } }
            }
        }
        if reviewing() {
            dialog { id: "prompt-review-dialog", class: "account-dialog prompt-review-dialog", "aria-labelledby": "prompt-review-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('prompt-review-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); if !*store.busy.peek() { reviewing.set(false); } },
                div { class: "section-heading", h2 { id: "prompt-review-title", {crate::i18n::tr("ตรวจรายการ")} } button { class: "icon-button", disabled: *store.busy.read(), "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| reviewing.set(false), Icon { name: "close", size: 20 } } }
                if let Some((true, error)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&error)}" } }
                div { class: "entry-editor",
                    if let Some((source, drafts)) = store.prompt_drafts.read().clone() { crate::prompt_review::PromptReview { source, drafts, view: view.clone() } }
                    else if let Some((source, drafts)) = store.batch.read().clone() { crate::batch_entry::BatchReview { source, drafts, view: view.clone() } }
                    else if let Some(prepared) = store.prepared.read().clone() { Confirmation { prepared, view: view.clone() } }
                    else if let Some((source, drafts)) = store.model_choices.read().clone() { crate::model::ModelChoices { source, drafts, view: view.clone() } }
                    else if let Some(input) = store.input.read().clone() { EntryForm { input, view: view.clone() } }
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
            div { h1 { {crate::i18n::text("เพิ่มรายการ", &[])} } p { class: "muted", {crate::i18n::text("กรอกข้อมูล แล้วตรวจให้ตรงก่อนบันทึก", &[])} } }

        }
        crate::navigation::EntryToolbar { manual: true }
        section { class: "card entry-editor manual-entry-editor",
            div { class: "composer-receipt-toolbar",
                ReceiptUpload { destination: ReceiptDestination::Manual }
                span { class: "field-hint", {crate::i18n::text("เลือกได้หลายรูป แล้วตรวจรายการด้านล่าง · สูงสุด 8 รูป", &[])} }
            }
            ReceiptAttachments {}
            if let Some((source, drafts)) = store.batch.read().clone() {
                crate::batch_entry::BatchReview { source, drafts, view: view.clone() }
            } else if view.accounts.is_empty() {
                EmptyState { title: crate::i18n::text("เพิ่มบัญชีก่อนเริ่มจด", &[]), body: crate::i18n::text("เลือกบัญชีที่จะใช้รับหรือจ่ายเงินก่อน", &[]) }
                button { class: "primary", onclick: move |_| store.account_form.set(true), {crate::i18n::text("เพิ่มบัญชีแรก", &[])} }
            } else if let Some(prepared) = prepared {
                Confirmation { prepared, view: view.clone() }
            } else if let Some(input) = input {
                EntryForm { input, view: view.clone() }
            } else {
                EmptyState { title: crate::i18n::text("พร้อมจดรายการถัดไป", &[]), body: crate::i18n::text("เพิ่มรายรับ รายจ่าย หรือโอนเงินได้ด้วยตัวเอง", &[]) }
                button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.new_entry(), Icon { name: "plus", size: 20 } {crate::i18n::text("เพิ่มรายการใหม่", &[])} }
            }
        }
    }
}

#[component]
fn EntryForm(input: EntryInput, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let initial_payment = input.kind == TransactionKind::Transfer
        && view.accounts.iter().any(|a| {
            Some(a.account.id()) == input.destination && a.account.kind() == AccountKind::CreditCard
        });
    let initial_full = credit_cards(&view).ok().is_some_and(|cards| {
        cards.iter().any(|c| {
            Some(c.account.id()) == input.destination && c.outstanding.to_string() == input.amount
        })
    });
    let mut payment = use_signal(move || initial_payment);
    let mut full = use_signal(move || initial_full);
    let is_payment = payment() && input.kind == TransactionKind::Transfer;
    let view_for_submit = view.clone();
    let funding = input.account.filter(|id| {
        view.accounts.iter().any(|a| {
            a.account.id() == *id
                && a.account.accepts_cash_entries()
                && a.account.kind() != AccountKind::CreditCard
        })
    });
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
        div { class: "section-heading", h2 { {crate::i18n::text("รายละเอียดรายการ", &[])} } span { class: "status-pill", {crate::i18n::text("ยังไม่บันทึก", &[])} } }
        div { class: "kind-tabs entry-kind-tabs", role: "group", "aria-label": crate::i18n::text("ชนิดรายการ", &[]),
            for (target, label) in [(TransactionKind::Expense, "รายจ่าย"), (TransactionKind::Income, "รายรับ"), (TransactionKind::Transfer, "โอนเงิน")] {
                button { class: if target == kind && !is_payment { "selected" } else { "" }, "aria-pressed": target == kind && !is_payment, disabled: *store.busy.read() || (has_receipt && target == TransactionKind::Transfer), onclick: move |_| { payment.set(false); store.update_entry(|i| { i.kind = target; i.category = None; i.destination = None; i.recurring = None; i.income_tax = None; }); }, "{crate::i18n::tr(&label)}" }
            }
            button { class: if is_payment { "selected" } else { "" }, "aria-pressed": is_payment, disabled: *store.busy.read() || has_receipt, onclick: move |_| {
                payment.set(true); full.set(true);
                store.update_entry(|i| { i.kind = TransactionKind::Transfer; i.income_tax = None; i.category = None; i.recurring = None; i.destination = None; i.account = funding; i.amount.clear(); });
            }, {crate::i18n::text("ชำระบัตรเครดิต", &[])} }
        }
        form { novalidate: true, onsubmit: move |event| { event.prevent_default(); let input = store.input.read().clone(); if let Some(input) = input {
                let fields = missing_entry_fields(&input);
                if fields.is_empty() {
                    if is_payment && let Err(error) = validate_credit_payment(&view_for_submit, &input) { store.notice.set(Some((true, error.to_string()))); return; }
                    store.send(Command::Preview(input));
                }
                else { store.notice.set(Some((true, crate::i18n::text("กรุณาเติมข้อมูลก่อนบันทึก: {0}", &[crate::i18n::field_list(&fields)])))); }
            } },
            if kind == TransactionKind::Expense {
                crate::recurring_picker::RecurringPicker { id: "entry-recurring", input: input.clone(), view: view.clone(), onchange: move |updated| store.update_entry(|input| *input = updated) }
            }
            if is_payment { crate::credit_payment::CreditPaymentFields { input: input.clone(), view: view.clone(), full } }
            label { r#for: "amount", {crate::i18n::text("จำนวนเงิน (บาท)", &[])} } input { id: "amount", class: "amount-input", inputmode: "decimal", required: true, maxlength: 18, placeholder: "0.00", value: input.amount.clone(), readonly: is_payment && full(), disabled: *store.busy.read(), oninput: move |event| store.update_entry(|i| i.amount = event.value()) }
            label { r#for: "entry-account", if kind == TransactionKind::Income { {crate::i18n::text("เงินเข้าบัญชี", &[])} } else { {crate::i18n::text("จากบัญชี", &[])} } }
            select { id: "entry-account", required: true, value: account_value, disabled: *store.busy.read(), onchange: move |event| store.update_entry(|i| i.account = event.value().parse().ok()),
                option { value: "", selected: input.account.is_none(), {crate::i18n::text("เลือกบัญชี", &[])} }
                for item in &view.accounts { if item.account.accepts_cash_entries() && (!is_payment || item.account.kind() != AccountKind::CreditCard) { option { value: "{item.account.id()}", selected: input.account == Some(item.account.id()), "{item.account.name().as_str()} · {crate::i18n::tr(item.account.kind().label())}" } } }
            }
            if kind == TransactionKind::Transfer {
                if !is_payment {
                label { r#for: "destination", {crate::i18n::text("ไปบัญชี", &[])} }
                select { id: "destination", required: true, value: destination_value, disabled: *store.busy.read(), onchange: move |event| store.update_entry(|i| i.destination = event.value().parse().ok()),
                    option { value: "", selected: input.destination.is_none(), {crate::i18n::text("เลือกบัญชีปลายทาง", &[])} }
                    for item in &view.accounts { if item.account.accepts_cash_entries() && Some(item.account.id()) != input.account { option { value: "{item.account.id()}", selected: input.destination == Some(item.account.id()), "{item.account.name().as_str()}" } } }
                }
                p { class: "field-hint", {crate::i18n::text("การโอนและการจ่ายยอดหนี้บัตรไม่เพิ่มรายรับรายจ่าย", &[])} }
                }
            } else {
                fieldset { class: "category-field", legend { {crate::i18n::text("หมวดหมู่", &[])} }
                    div { class: "category-grid", for category in categories { { let category = *category; rsx! {
                        button { r#type: "button", class: if input.category == Some(category) { "category-tile selected" } else { "category-tile" },
                            "aria-pressed": input.category == Some(category), disabled: *store.busy.read(), onclick: move |_| store.update_entry(|i| i.category = Some(category)),
                            ArtIcon { name: category_art(category), size: 42 } span { "{crate::i18n::tr(category.label())}" }
                        }
                    } } } }
                }
            }
            if kind == TransactionKind::Income && view.thai_tax_enabled && view.currency == Currency::Thb {
                crate::income_tax::IncomeTaxFields { input: input.clone(), id: "entry-tax", onchange: move |updated| store.update_entry(|input| *input = updated) }
            }
            label { r#for: "entry-date", {crate::i18n::text("วันที่รายการ", &[])} } input { id: "entry-date", r#type: "date", required: true, min: "1900-01-01", max: "{view.today}", value: input.date, disabled: *store.busy.read(), onchange: move |event| store.update_entry(|i| i.date = event.value()) }
            if let Some(receipt) = input.receipt.clone() { crate::receipt_editor::ReceiptEditor { receipt, total: input.amount.clone(), id: "entry-receipt", onchange: move |receipt| store.update_entry(|input| input.receipt = Some(receipt)) } }
            label { r#for: "entry-note", {crate::i18n::text("รายละเอียดเพิ่มเติม (ไม่จำเป็น)", &[])} } textarea { id: "entry-note", maxlength: 500, rows: "3", placeholder: crate::i18n::text("เช่น ข้าวกลางวัน", &[]), value: input.note, disabled: *store.busy.read(), oninput: move |event| store.update_entry(|i| i.note = event.value()) }
            if !can_preview { p { class: "batch-question", role: "status", {crate::i18n::text("ยังขาด: {0} — เติมข้อมูลเหล่านี้ก่อนบันทึก", &[crate::i18n::field_list(&missing)])} } }
            button { class: "primary full-width", r#type: "submit", disabled: *store.busy.read(), if can_preview { {crate::i18n::text("ตรวจรายการก่อนบันทึก", &[])} } else { {crate::i18n::text("ตรวจข้อมูลที่ยังขาด", &[])} } Icon { name: "arrow", size: 18 } }
        }
    }
}

#[component]
fn Confirmation(prepared: PreparedEntry, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let Some((label, amount, account)) = entry_label(&prepared.entry) else {
        return rsx! { p { role: "alert", {crate::i18n::text("ไม่สามารถแสดงรายการนี้ได้", &[])} } };
    };
    let account = account_label(&view, account);
    let label = if matches!(prepared.entry.kind(), EntryKind::Transfer { to, .. } if view.accounts.iter().any(|a| a.account.id() == *to && a.account.kind() == AccountKind::CreditCard))
    {
        "ชำระบัตรเครดิต"
    } else {
        label
    };
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
            span { class: "confirm-symbol", Icon { name: "check", size: 30 } } h2 { {crate::i18n::text("ตรงกับรายการของเราไหม?", &[])} }
            strong { class: "confirm-amount", "{crate::i18n::currency_prefix()}{money_label(amount)}" }
            dl { div { dt { {crate::i18n::text("รายการ", &[])} } dd { "{crate::i18n::tr(&label)}" } } div { dt { {crate::i18n::text("บัญชี", &[])} } dd { "{account}" } } if let Some(destination) = destination { div { dt { {crate::i18n::text("ปลายทาง", &[])} } dd { "{destination}" } } } div { dt { {crate::i18n::text("วันที่", &[])} } dd { "{prepared.entry.date()}" } } if !prepared.entry.note().as_str().is_empty() { div { class: "confirmation-note", dt { {crate::i18n::text("รายละเอียด", &[])} } dd { "{prepared.entry.note().as_str()}" } } } }
            if let Some(link) = &prepared.recurring { p { class: "batch-question", {crate::i18n::text("ผูกกับ {0} · {1} · จะนับจ่ายหนึ่งงวดโดยไม่ลงรายจ่ายซ้ำ", &[link.schedule.name().as_str().to_string(), crate::recurring_picker::installment_label(&link.schedule, link.month).to_string()])} } }
            if let Some(tax) = prepared.entry.income_tax() { crate::income_tax::IncomeTaxSummary { tax } }
            if salary && prepared.entry.income_tax().is_none() { p { class: "inline-warning", {crate::i18n::text("นี่คือยอดเงินที่รับเข้าบัญชี ยังไม่ใช้แทนเงินเดือนก่อนหักสำหรับคำนวณภาษี", &[])} } }
            p { class: "field-hint", {crate::i18n::text("ยังไม่ได้บันทึก กดยืนยันเมื่อรายละเอียดถูกต้อง", &[])} }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.send(Command::Commit(prepared_for_save.clone())), if *store.busy.read() { {crate::i18n::text("กำลังบันทึก…", &[])} } else { {crate::i18n::text("ยืนยันบันทึก", &[])} } }
            button { class: "text-button full-width", disabled: *store.busy.read(), onclick: move |_| store.prepared.set(None), {crate::i18n::text("กลับไปแก้ไข", &[])} }
        }
    }
}
