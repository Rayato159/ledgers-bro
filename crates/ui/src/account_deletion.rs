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
            h2 { id: "delete-account-title", {crate::i18n::text("ลบบัญชี {0}?", &[deletion.account().name().as_str().to_string()])} }
            p { id: "delete-account-warning", {crate::i18n::text("บัญชี ยอดเริ่มต้น และรายการที่เกี่ยวข้องจะถูกลบถาวร ย้อนกลับไม่ได้", &[])} }
            ul { class: "deletion-summary",
                li { {crate::i18n::text("รายรับ รายจ่าย และรายการโอน: {0} รายการ", &[format!("{}", deletion.transaction_count())])} }
                li { {crate::i18n::text("ในนี้มีรายการโอนระหว่างบัญชี: {0} รายการ", &[format!("{}", deletion.transfer_count())])} }
                li { {crate::i18n::text("ประวัติการยกเลิกที่ลบด้วย: {0} รายการ", &[format!("{}", deletion.reversal_count())])} }
            }
            if deletion.affected_recurring_count() > 0 {
                p { class: "deletion-transfer-warning", {crate::i18n::text("มีแผนรายจ่ายประจำที่เกี่ยวข้อง {0} แผน แผนยังอยู่แต่ต้องเลือกบัญชีใหม่หากบัญชีเดิมถูกลบ งวดที่ผูกกับรายจ่ายที่ลบจะกลับเป็นค้างจ่าย", &[format!("{}", deletion.affected_recurring_count())])} }
            }
            if !deletion.affected_accounts().is_empty() {
                p { class: "deletion-transfer-warning", {crate::i18n::text("รายการโอนที่เกี่ยวข้องจะหายไปจากทั้งสองบัญชี ยอดของบัญชีที่เหลือจะคำนวณใหม่ดังนี้", &[])} }
                ul { class: "deletion-impacts",
                    for impact in deletion.affected_accounts() {
                        {
                            let credit = impact.account.kind() == AccountKind::CreditCard;
                            let before = if credit { impact.before.negated() } else { impact.before };
                            let after = if credit { impact.after.negated() } else { impact.after };
                            rsx! { li {
                                strong { "{impact.account.name().as_str()}" }
                                span { if credit { {crate::i18n::text("ยอดหนี้ (ติดลบ = จ่ายเกิน)", &[])} } else { {crate::i18n::text("ยอดคงเหลือ", &[])} } }
                                span { "{crate::i18n::currency_prefix()}{money_label(before)} → {crate::i18n::currency_prefix()}{money_label(after)}" }
                            } }
                        }
                    }
                }
            }
            if let Some((_, message)) = error { p { class: "form-error", role: "alert", "{message}" } }
            div { class: "deletion-actions",
                button { class: "soft-button", autofocus: true, disabled: *store.busy.read(),
                    onclick: move |_| store.account_deletion.set(None), {crate::i18n::text("เก็บบัญชีไว้", &[])} }
                button { class: "text-button", disabled: *store.busy.read(),
                    onclick: move |_| store.send(Command::PreviewDeleteAccount(id)), {crate::i18n::text("ตรวจสอบรายการใหม่", &[])} }
                button { class: "danger-button delete-account-confirm", disabled: blocked,
                    onclick: move |_| store.send(Command::DeleteAccount(deletion.clone())),
                    Icon { name: "trash", size: 20 }
                    if *store.busy.read() { {crate::i18n::text("กำลังดำเนินการ…", &[])} } else { {crate::i18n::text("ลบบัญชีและรายการทั้งหมด", &[])} }
                }
            }
        }
    }
}
