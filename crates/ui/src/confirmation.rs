use crate::{components::Icon, i18n::tr};
use dioxus::prelude::*;

#[derive(Clone)]
pub struct PendingConfirmation {
    details: String,
    action: EventHandler,
}

#[derive(Clone, Copy)]
pub struct Confirmations(pub Signal<Option<PendingConfirmation>>);

/// Keep the proposed values in the closure until the user explicitly confirms.
/// Cancelling drops that closure without issuing any storage command.
pub fn ask(details: String, action: impl FnMut(()) + 'static) {
    let Some(mut confirmations) = try_consume_context::<Confirmations>() else {
        return;
    };
    if confirmations.0.peek().is_none() {
        confirmations.0.set(Some(PendingConfirmation {
            details,
            action: EventHandler::new(action),
        }));
    }
}

#[component]
pub fn ConfirmationAlert() -> Element {
    let mut confirmations = use_context::<Confirmations>();
    let pending = confirmations.0.read().clone();
    rsx! {
        if let Some(pending) = pending {
            dialog { id: "edit-confirmation", class: "account-dialog confirmation-alert", role: "alertdialog", "aria-labelledby": "edit-confirmation-title", "aria-describedby": "edit-confirmation-copy",
                onmounted: move |_| { let _ = document::eval("document.getElementById('edit-confirmation').showModal()"); },
                oncancel: move |e| { e.prevent_default(); confirmations.0.set(None); },
                h2 { id: "edit-confirmation-title", {tr("ยืนยันการแก้ไข?")} }
                p { id: "edit-confirmation-copy", class: "entry-note-body", "{pending.details}" }
                p { class: "field-hint", {tr("ยืนยันเพื่อบันทึกและโหลดข้อมูลล่าสุดอัตโนมัติ หรือยกเลิกเพื่อกลับไปแก้ไข")} }
                div { class: "dialog-actions",
                    button { r#type: "button", class: "primary", onclick: move |_| {
                        let pending = confirmations.0.write().take();
                        if let Some(pending) = pending {
                            pending.action.call(());
                        }
                    }, Icon { name: "check", size: 18 } {tr("ยืนยัน")} }
                    button { r#type: "button", class: "soft-button", autofocus: true, onclick: move |_| confirmations.0.set(None), {tr("ยกเลิก")} }
                }
            }
        }
    }
}
