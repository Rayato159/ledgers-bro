use crate::{HostInfo, artwork::*, components::*, state::*};
use dioxus::prelude::*;
use ledger_application::{Command, Dashboard};
use ledger_domain::*;

#[component]
pub fn AccountsPage(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let host = use_context::<HostInfo>();
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("บัญชีของเรา", &[])} } p { class: "muted", {crate::i18n::text("{0} / 100 บัญชี · ทุกยอดเป็นเงินบาท", &[format!("{}", view.accounts.len())])} } } button { class: "primary", disabled: view.accounts.len() >= 100 || *store.busy.read(), onclick: move |_| store.account_form.set(true), Icon { name: "plus", size: 17 } {crate::i18n::text("เพิ่มบัญชี", &[])} } }
        div { class: "account-art-banner card", div { h2 { {crate::i18n::text("เงินแต่ละก้อน อยู่ตรงไหน", &[])} } p { {crate::i18n::text("แยกบัญชีตามที่ใช้ แล้วค่อย ๆ ดูแลไปด้วยกัน", &[])} }
            div { class: "account-art-types", for kind in AccountKind::ALL { span { title: crate::i18n::tr(kind.label()), ArtIcon { name: account_art(kind), size: 44 } } } }
        } img { src: host.art.hero.clone(), alt: crate::i18n::text("ทานูกิถือสมุดบัญชี", &[]) } }
        if view.accounts.is_empty() { div { class: "card", EmptyState { title: crate::i18n::text("เริ่มจากกระเป๋าใบแรก", &[]).to_owned(), body: crate::i18n::text("เพิ่มเงินสดหรือธนาคาร แล้วใส่ยอดที่มีอยู่ตอนนี้ ยอดเริ่มต้นจะไม่นับเป็นรายรับ", &[]).to_owned() } } }
        div { class: "account-grid",
            for item in &view.accounts {
                {
                    let is_credit = item.account.kind() == AccountKind::CreditCard;
                    let id = item.account.id();
                    let display = if is_credit { item.balance.negated() } else { item.balance };
                    rsx! { article { class: "card account-card", key: "{item.account.id()}",
                        span { class: "account-icon", ArtIcon { name: account_art(item.account.kind()), size: 48 } }
                        small { "{crate::i18n::tr(item.account.kind().label())}" } h2 { "{item.account.name().as_str()}" }
                        strong { "{crate::i18n::currency_prefix()}{money_label(display)}" }
                        p { class: "muted small", if is_credit { {crate::i18n::text("ยอดหนี้คงเหลือ (ติดลบ = จ่ายเกิน)", &[])} } else { {crate::i18n::text("ยอดคงเหลือ", &[])} } }
                        button { class: "danger-button delete-account-button", disabled: *store.busy.read(),
                            "aria-label": crate::i18n::text("ลบบัญชี {0}", &[item.account.name().as_str().to_string()]),
                            onclick: move |_| store.send(Command::PreviewDeleteAccount(id)),
                            Icon { name: "trash", size: 20 } {crate::i18n::text("ลบบัญชี", &[])}
                        }
                    } }
                }
            }
        }
        crate::debt_visuals::CreditCardsPanel { view: view.clone() }
        div { class: "inline-note", Icon { name: "file", size: 19 } p { {crate::i18n::text("คริปโตและพอร์ตหุ้นในรุ่นนี้เป็นการจดมูลค่าเงินบาทด้วยมือ ยังไม่มีราคาตลาดสดหรือการซื้อขายสินทรัพย์", &[])} } }
    }
}

#[component]
pub fn AccountDialog() -> Element {
    let mut store = use_context::<UiState>();
    let mut name = use_signal(String::new);
    let mut kind = use_signal(|| AccountKind::Cash);
    let mut opening = use_signal(|| "0".to_owned());
    let closing = use_signal(String::new);
    let payment = use_signal(String::new);
    rsx! {
        dialog { id: "account-dialog", class: "account-dialog", "aria-labelledby": "account-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('account-dialog').showModal()"); },
            oncancel: move |event| { event.prevent_default(); if !*store.busy.read() { store.account_form.set(false); } },
            form { onsubmit: move |event| {
                event.prevent_default();
                let credit_cycle = if kind() == AccountKind::CreditCard {
                    match crate::debt_visuals::cycle_from_fields(&closing(), &payment()) {
                        Ok(cycle) => Some(cycle),
                        Err(e) => { store.notice.set(Some((true, e.to_string()))); return; }
                    }
                } else { None };
                store.send(Command::CreateAccount { name: name(), kind: kind(), opening: opening(), credit_cycle });
            },
                div { class: "section-heading", h2 { id: "account-title", {crate::i18n::text("เพิ่มบัญชีใหม่", &[])} } button { r#type: "button", class: "icon-button", "aria-label": crate::i18n::text("ปิดหน้าต่าง", &[]), disabled: *store.busy.read(), onclick: move |_| store.account_form.set(false), Icon { name: "close", size: 20 } } }
                p { class: "muted", {crate::i18n::text("ให้เงินแต่ละก้อนมีที่ของมัน", &[])} }
                label { r#for: "account-name", {crate::i18n::text("ชื่อบัญชี", &[])} } input { id: "account-name", name: "account-name", autofocus: true, required: true, maxlength: 60, placeholder: crate::i18n::text("เช่น เงินสด หรือ ธนาคาร ออมเงิน", &[]), value: "{name}", disabled: *store.busy.read(), oninput: move |event| name.set(event.value()) }
                label { r#for: "account-kind", {crate::i18n::text("ประเภทบัญชี", &[])} } select { id: "account-kind", value: kind().code(), disabled: *store.busy.read(), onchange: move |event| { if let Ok(value) = AccountKind::from_code(&event.value()) { kind.set(value); } }, for item in AccountKind::ALL { option { value: item.code(), "{crate::i18n::tr(item.label())}" } } }
                div { class: "account-kind-preview", ArtIcon { name: account_art(kind()), size: 44 } span { "{crate::i18n::tr(kind().label())}" } }
                label { r#for: "opening", if kind() == AccountKind::CreditCard { {crate::i18n::text("ยอดหนี้ที่ค้างอยู่ (ไม่ใช่วงเงินบัตร)", &[])} } else { {crate::i18n::text("ยอดเริ่มต้น (บาท)", &[])} } } input { id: "opening", name: "opening", inputmode: "decimal", required: true, value: "{opening}", disabled: *store.busy.read(), oninput: move |event| opening.set(event.value()) }
                p { class: "field-hint", {crate::i18n::text("ยอดเริ่มต้นไม่ถูกนับเป็นรายรับเดือนนี้", &[])} }
                if kind() == AccountKind::CreditCard {
                    fieldset { class: "credit-cycle-group", disabled: *store.busy.read(), "aria-label": crate::i18n::tr("บัตรเครดิต"), crate::debt_visuals::CycleFields { closing, payment, prefix: "new-card" } }
                    p { class: "field-hint", {crate::i18n::text("ยอดหนี้เริ่มต้นรวมในยอดรอชำระ แต่ไม่เดารอบบิลย้อนหลัง", &[])} }
                }
                if let Some((true, message)) = store.notice.read().clone() { p { class: "form-error", role: "alert", "{message}" } }
                button { class: "primary full-width", r#type: "submit", disabled: *store.busy.read(), if *store.busy.read() { {crate::i18n::text("กำลังบันทึก…", &[])} } else { {crate::i18n::text("สร้างบัญชี", &[])} } }
            }
        }
    }
}

#[component]
pub fn TransactionsPage(view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let host = use_context::<HostInfo>();
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("รายการทั้งหมด", &[])} } p { class: "muted", {crate::i18n::text("เรียงตามเวลาที่บันทึก • ยกเลิกรายการได้โดยเก็บประวัติไว้", &[])} } } button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.notice.set(None); store.export_form.set(true); }, Icon { name: "download", size: 18 } {crate::i18n::text("ส่งออก CSV", &[])} } }
        div { class: "history-art-banner", img { src: host.art.hero.clone(), alt: crate::i18n::text("ทานูกิช่วยดูสมุดบัญชี", &[]) } div { h2 { {crate::i18n::text("ทุกรายการ ตรวจดูได้", &[])} } p { {crate::i18n::text("รายรับ รายจ่าย และการโอน รวมไว้ในที่เดียว", &[])} } } }
        section { class: "card transaction-card", TransactionRows { view, limit: usize::MAX, allow_cancel: true } }
    }
}

#[component]
pub fn TransactionRows(view: Dashboard, limit: usize, allow_cancel: bool) -> Element {
    let store = use_context::<UiState>();
    let mut cancelling = use_signal(|| None::<EntryId>);
    let entries: Vec<_> = view
        .entries
        .iter()
        .filter(|entry| {
            !matches!(
                entry.kind(),
                EntryKind::Opening { .. }
                    | EntryKind::ReceivableOpening { .. }
                    | EntryKind::Reversal { .. }
            )
        })
        .take(limit)
        .collect();
    rsx! {
        if entries.is_empty() { EmptyState { title: crate::i18n::text("หน้านี้รอเรื่องราวของเราอยู่", &[]).to_owned(), body: crate::i18n::text("รายรับ รายจ่าย และการโอนที่บันทึกจะแสดงตรงนี้", &[]).to_owned() } }
        for entry in entries {
            if let Some((label, amount, account)) = entry_label(entry) {
                {
                    let id = entry.id();
                    let cancelled = view.reversed.contains(&id);
                    let income = matches!(entry.kind(), EntryKind::Income { .. } | EntryKind::Repayment { .. });
                    let transfer = matches!(entry.kind(), EntryKind::Transfer { .. });
                    let account_name = account_label(&view, account);
                    let icon_name = entry_art(entry.kind());
                    let sign = if income { "+" } else if !transfer { "−" } else { "" };
                    let note = entry.note().as_str();
                    let title = if note.is_empty() { crate::i18n::tr(label) } else { note.lines().next().unwrap_or(label).to_owned() };
                    let destination = match entry.kind() { EntryKind::Transfer { to, .. } => format!(" → {}", account_label(&view, *to)), _ => String::new() };
                    rsx! { article { class: if cancelled { "transaction-row cancelled" } else { "transaction-row" }, key: "{id}",
                        span { class: if income { "row-icon incoming" } else { "row-icon" }, ArtIcon { name: icon_name, size: 37 } }
                        div { class: "transaction-description", strong { "{title}" } small { "{entry.date()} · {crate::i18n::tr(&label)} · {account_name}{destination}" } if note.contains('\n') { details { class: "saved-entry-details", summary { {crate::i18n::text("ดูรายละเอียด", &[])} } div { class: "entry-note-body", "{note}" } } } if cancelled { span { class: "cancel-tag", {crate::i18n::text("ยกเลิกแล้ว", &[])} } } }
                        strong { class: if income { "entry-amount positive" } else { "entry-amount" }, "{sign}{crate::i18n::currency_prefix()}{money_label(amount)}" }
                        if allow_cancel && !cancelled {
                            if cancelling() == Some(id) {
                                div { class: "cancel-confirm", span { {crate::i18n::text("ยกเลิกรายการนี้?", &[])} } button { class: "danger-button", disabled: *store.busy.read(), onclick: move |_| { store.send(Command::Reverse(id)); cancelling.set(None); }, {crate::i18n::text("ยืนยันยกเลิก", &[])} } button { class: "text-button", onclick: move |_| cancelling.set(None), {crate::i18n::text("กลับ", &[])} } }
                            } else { button { class: "text-button cancel-action", disabled: *store.busy.read(), onclick: move |_| cancelling.set(Some(id)), {crate::i18n::text("ยกเลิก", &[])} } }
                        }
                    } }
                }
            }
        }
    }
}
