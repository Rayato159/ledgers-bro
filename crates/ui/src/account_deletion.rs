use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::{AccountDeletion, Command};
use ledger_domain::AccountKind;

#[component]
pub fn DeleteAccountDialog(deletion: AccountDeletion) -> Element {
    let mut store = use_context::<UiState>();
    let id = deletion.account().id();
    let error = store
        .notice
        .read()
        .clone()
        .filter(|(is_error, _)| *is_error);
    let blocked = *store.busy.read() || error.is_some();
    rsx! {
        dialog { id: "delete-account-dialog", class: "account-dialog delete-account-dialog",
            "aria-labelledby": "delete-account-title", "aria-describedby": "delete-account-warning",
            onmounted: move |_| { let _ = document::eval("document.getElementById('delete-account-dialog').showModal()"); },
            oncancel: move |event| { event.prevent_default(); if !*store.busy.read() { store.account_deletion.set(None); } },
            h2 { id: "delete-account-title", "ลบบัญชี {deletion.account().name().as_str()}?" }
            p { id: "delete-account-warning", "บัญชี ยอดเริ่มต้น และรายการที่เกี่ยวข้องจะถูกลบถาวร ย้อนกลับไม่ได้" }
            ul { class: "deletion-summary",
                li { "รายรับ รายจ่าย และรายการโอน: {deletion.transaction_count()} รายการ" }
                li { "ในนี้มีรายการโอนระหว่างบัญชี: {deletion.transfer_count()} รายการ" }
                li { "ประวัติการยกเลิกที่ลบด้วย: {deletion.reversal_count()} รายการ" }
            }
            if !deletion.affected_accounts().is_empty() {
                p { class: "deletion-transfer-warning", "รายการโอนที่เกี่ยวข้องจะหายไปจากทั้งสองบัญชี ยอดของบัญชีที่เหลือจะคำนวณใหม่ดังนี้" }
                ul { class: "deletion-impacts",
                    for impact in deletion.affected_accounts() {
                        {
                            let credit = impact.account.kind() == AccountKind::CreditCard;
                            let before = if credit { impact.before.negated() } else { impact.before };
                            let after = if credit { impact.after.negated() } else { impact.after };
                            rsx! { li {
                                strong { "{impact.account.name().as_str()}" }
                                span { if credit { "ยอดหนี้ (ติดลบ = จ่ายเกิน)" } else { "ยอดคงเหลือ" } }
                                span { "฿{money_label(before)} → ฿{money_label(after)}" }
                            } }
                        }
                    }
                }
            }
            if let Some((_, message)) = error { p { class: "form-error", role: "alert", "{message}" } }
            div { class: "deletion-actions",
                button { class: "soft-button", autofocus: true, disabled: *store.busy.read(),
                    onclick: move |_| store.account_deletion.set(None), "เก็บบัญชีไว้" }
                button { class: "text-button", disabled: *store.busy.read(),
                    onclick: move |_| store.send(Command::PreviewDeleteAccount(id)), "ตรวจสอบรายการใหม่" }
                button { class: "danger-button delete-account-confirm", disabled: blocked,
                    onclick: move |_| store.send(Command::DeleteAccount(deletion.clone())),
                    Icon { name: "trash", size: 20 }
                    if *store.busy.read() { "กำลังดำเนินการ…" } else { "ลบบัญชีและรายการทั้งหมด" }
                }
            }
        }
    }
}
