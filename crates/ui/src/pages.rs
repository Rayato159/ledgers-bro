use crate::{artwork::*, components::*, state::*};
use dioxus::prelude::*;
use ledger_application::{Command, Dashboard};
use ledger_domain::*;

#[component]
pub fn AccountsPage(view: Dashboard) -> Element {
    let tab = use_signal(|| 0usize);
    let mut store = use_context::<UiState>();
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("บัญชีของเรา", &[])} } p { class: "muted", {crate::i18n::text("{0} / 100 บัญชี · มูลค่ารวมแสดงเป็นเงินบาท", &[format!("{}", view.accounts.len())])} } } button { class: "primary", disabled: view.accounts.len() >= 100 || *store.busy.read(), onclick: move |_| store.account_form.set(true), Icon { name: "plus", size: 17 } {crate::i18n::text("เพิ่มบัญชี", &[])} } }
        if view.accounts.is_empty() { div { class: "card", EmptyState { title: crate::i18n::text("เริ่มจากกระเป๋าใบแรก", &[]).to_owned(), body: crate::i18n::text("เพิ่มเงินสดหรือธนาคาร แล้วใส่ยอดที่มีอยู่ตอนนี้ ยอดเริ่มต้นจะไม่นับเป็นรายรับ", &[]).to_owned() } } }
        PageTabs { id: "accounts", tabs: vec![("wallet", "เงินสดและธนาคาร"), ("file", "บัญชีบัตรเครดิต"), ("up", "พอร์ตลงทุน"), ("calendar", "รอบบิลและการชำระ")], selected: tab }
        for group in 0..3 {
        PagePanel { lazy: true, id: "accounts", index: group, selected: tab(),
        div { class: "account-grid",
            for item in view.accounts.iter().filter(|a| match group { 0 => matches!(a.account.kind(), AccountKind::Cash | AccountKind::Bank), 1 => a.account.kind() == AccountKind::CreditCard, _ => matches!(a.account.kind(), AccountKind::Crypto | AccountKind::Investment) }) {
                if item.account.kind() == AccountKind::Crypto {
                    crate::crypto::CryptoAccountCard { key: "{item.account.id()}", account: item.account.clone(), book_balance: item.balance }
                } else {
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
        }
        if !view.accounts.iter().any(|a| match group { 0 => matches!(a.account.kind(), AccountKind::Cash | AccountKind::Bank), 1 => a.account.kind() == AccountKind::CreditCard, _ => matches!(a.account.kind(), AccountKind::Crypto | AccountKind::Investment) }) { p { class: "muted", {crate::i18n::tr("ยังไม่มีบัญชีในหมวดนี้")} } }
        if group == 2 {
            if view.accounts.iter().any(|a| a.account.kind() == AccountKind::Crypto) { div { class: "bottom-market", crate::crypto::CryptoMarketStatus {} } }
            div { class: "inline-note", Icon { name: "file", size: 19 } p { {crate::i18n::tr("พอร์ตหุ้นยังใช้มูลค่าที่บันทึกด้วยมือ ส่วนคริปโตใช้จำนวนเหรียญและราคาตลาด ไม่มีการส่งคำสั่งซื้อขาย")} } }
        }
        } }
        PagePanel { lazy: true, id: "accounts", index: 3, selected: tab(), crate::debt_visuals::CreditCardsPanel { view: view.clone() } }
    }
}

#[component]
pub fn AccountDialog() -> Element {
    let mut store = use_context::<UiState>();
    let mut name = use_signal(String::new);
    let mut kind = use_signal(|| AccountKind::Cash);
    let mut opening = use_signal(|| "0".to_owned());
    let bitcoin = use_signal(|| "0".to_owned());
    let solana = use_signal(|| "0".to_owned());
    let closing = use_signal(String::new);
    let payment = use_signal(String::new);
    rsx! {
        dialog { id: "account-dialog", class: "account-dialog", "aria-labelledby": "account-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('account-dialog').showModal()"); },
            oncancel: move |event| { event.prevent_default(); if !*store.busy.read() { store.account_form.set(false); } },
            form { onsubmit: move |event| {
                event.prevent_default();
                if kind() == AccountKind::Crypto {
                    match CryptoHoldings::parse(&bitcoin(), &solana()) {
                        Ok(holdings) => store.send(Command::CreateCryptoAccount { name: name(), holdings }),
                        Err(e) => store.notice.set(Some((true, e.to_string()))),
                    }
                    return;
                }
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
                if kind() == AccountKind::Crypto {
                    crate::crypto::CryptoAmounts { bitcoin, solana }
                    if store.view.read().as_ref().is_some_and(|v| v.currency != Currency::Thb) { p { class: "form-error", {crate::i18n::tr("พอร์ตเหรียญรองรับเฉพาะสมุดบัญชี THB")} } }
                } else {
                    label { r#for: "opening", if kind() == AccountKind::CreditCard { {crate::i18n::text("ยอดหนี้ที่ค้างอยู่ (ไม่ใช่วงเงินบัตร)", &[])} } else { {crate::i18n::text("ยอดเริ่มต้น (บาท)", &[])} } } input { id: "opening", name: "opening", inputmode: "decimal", required: true, value: "{opening}", disabled: *store.busy.read(), oninput: move |event| opening.set(event.value()) }
                    p { class: "field-hint", {crate::i18n::text("ยอดเริ่มต้นไม่ถูกนับเป็นรายรับเดือนนี้", &[])} }
                }
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
    let tab = use_signal(|| 0usize);
    let mut year = use_signal(|| view.today.month_key().0);
    let years: std::collections::BTreeSet<_> = view
        .entries
        .iter()
        .map(|e| e.date().month_key().0)
        .chain([view.today.month_key().0])
        .collect();
    let mut page = use_signal(|| 0usize);
    use_effect(move || {
        let _ = (tab(), year());
        page.set(0);
    });
    let filtered: Vec<_> = view
        .entries
        .iter()
        .filter(|entry| {
            (year() == 0 || entry.date().month_key().0 == year())
                && match entry.kind() {
                    EntryKind::Opening { .. }
                    | EntryKind::ReceivableOpening { .. }
                    | EntryKind::Reversal { .. } => false,
                    kind => match tab() {
                        1 => matches!(kind, EntryKind::Expense { .. }),
                        2 => matches!(kind, EntryKind::Income { .. }),
                        3 => matches!(kind, EntryKind::Transfer { .. }),
                        4 => matches!(
                            kind,
                            EntryKind::Lending { .. } | EntryKind::Repayment { .. }
                        ),
                        _ => true,
                    },
                }
        })
        .cloned()
        .collect();
    let pages = filtered.len().div_ceil(20).max(1);
    let current_page = page().min(pages - 1);
    let mut visible = view.clone();
    visible.entries = filtered
        .into_iter()
        .skip(current_page * 20)
        .take(20)
        .collect();
    let mut store = use_context::<UiState>();
    rsx! {
        section { class: "page-heading", div { h1 { {crate::i18n::text("รายการทั้งหมด", &[])} } p { class: "muted", {crate::i18n::text("เรียงตามเวลาที่บันทึก • ยกเลิกรายการได้โดยเก็บประวัติไว้", &[])} } } button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| { store.notice.set(None); store.export_form.set(true); }, Icon { name: "download", size: 18 } {crate::i18n::text("ส่งออก CSV", &[])} } }
        PageTabs { id: "transactions", tabs: vec![("list", "ทั้งหมด"), ("up", "รายจ่าย"), ("down", "รายรับ"), ("arrow", "โอนเงิน"), ("user", "เงินให้ยืมและรับคืน")], selected: tab }
        div { class: "history-year-filter", label { r#for: "history-year", {crate::i18n::tr("ปีที่แสดง")} }
            select { id: "history-year", value: "{year}", onchange: move |e| { if let Ok(value) = e.value().parse() { year.set(value); } },
                option { value: "0", {crate::i18n::tr("ทุกปี")} }
                for value in years.into_iter().rev() { option { value: "{value}", "{crate::i18n::year(value)}" } }
            }
        }
        section { id: "transactions-panel-{tab}", class: "card transaction-card", role: "tabpanel", "aria-labelledby": "transactions-tab-{tab}", TransactionRows { key: "{tab}-{current_page}", view: visible, limit: 20, allow_cancel: true } }
        if pages > 1 { div { class: "list-pagination",
            button { class: "soft-button", disabled: current_page == 0, onclick: move |_| page.set(current_page.saturating_sub(1)), {crate::i18n::tr("ก่อนหน้า")} }
            span { {crate::i18n::text("หน้า {0} / {1}", &[(current_page + 1).to_string(), pages.to_string()])} }
            button { class: "soft-button", disabled: current_page + 1 >= pages, onclick: move |_| page.set(current_page + 1), {crate::i18n::tr("ถัดไป")} }
        } }
    }
}

#[component]
pub fn TransactionRows(view: Dashboard, limit: usize, allow_cancel: bool) -> Element {
    let store = use_context::<UiState>();
    let mut cancelling = use_signal(|| None::<EntryId>);
    let mut inspecting = use_signal(|| None::<EntryId>);
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
                        div { class: "transaction-description",
                            div { class: "transaction-title-line",
                                button { class: "transaction-title", title: "{title}", "aria-haspopup": "dialog", onclick: move |_| inspecting.set(Some(id)), strong { "{title}" } }
                                if cancelled { span { class: "cancel-tag", {crate::i18n::text("ยกเลิกแล้ว", &[])} } }
                                else if allow_cancel { button { class: "text-button cancel-action", disabled: *store.busy.read(), onclick: move |_| cancelling.set(Some(id)), {crate::i18n::tr("ยกเลิก")} } }
                            }
                            small { title: "{entry.date()} · {crate::i18n::tr(label)} · {account_name}{destination}", "{entry.date()} · {crate::i18n::tr(label)} · {account_name}{destination}" }
                        }
                        strong { class: if income { "entry-amount positive" } else { "entry-amount" }, "{sign}{crate::i18n::currency_prefix()}{money_label(amount)}" }
                    } }
                }
            }
        }
        if let Some(id) = cancelling() {
            dialog { id: "cancel-entry-dialog", class: "account-dialog", "aria-label": crate::i18n::tr("ยกเลิกรายการนี้?"),
                onmounted: move |_| { let _ = document::eval("document.getElementById('cancel-entry-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); cancelling.set(None); },
                h2 { {crate::i18n::tr("ยกเลิกรายการนี้?")} }
                div { class: "dialog-actions",
                    button { class: "danger-button", disabled: (store.busy)(), onclick: move |_| { store.send(Command::Reverse(id)); cancelling.set(None); }, {crate::i18n::tr("ยืนยันยกเลิก")} }
                    button { class: "soft-button", onclick: move |_| cancelling.set(None), {crate::i18n::tr("กลับ")} }
                }
            }
        }
        if let Some(id) = inspecting() { if let Some(entry) = view.entries.iter().find(|e| e.id() == id) {
            dialog { id: "transaction-details", class: "account-dialog", "aria-labelledby": "transaction-details-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('transaction-details').showModal()"); },
                oncancel: move |e| { e.prevent_default(); inspecting.set(None); },
                div { class: "section-heading", h2 { id: "transaction-details-title", {crate::i18n::tr("ดูรายละเอียด")} } button { class: "icon-button", "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| inspecting.set(None), Icon { name: "close", size: 20 } } }
                if let Some((label, amount, account)) = entry_label(entry) {
                    p { "{entry.date()} · {crate::i18n::tr(label)} · {account_label(&view, account)}" }
                    strong { "{crate::i18n::currency_prefix()}{money_label(amount)}" }
                }
                if view.reversed.contains(&id) { p { class: "cancel-tag", {crate::i18n::tr("ยกเลิกแล้ว")} } }
                p { class: "entry-note-body", "{entry.note().as_str()}" }
                if let Some(tax) = entry.income_tax() { crate::income_tax::IncomeTaxSummary { tax } }
                crate::entry_breakdown::EntryBreakdown { view: view.clone(), entry: entry.clone() }
            }
        } }
    }
}
