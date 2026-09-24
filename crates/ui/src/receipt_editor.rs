use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::{ReceiptInput, ReceiptLineInput};
use ledger_domain::{MAX_RECEIPT_LINES, ReceiptLineKind};

#[component]
pub fn ReceiptEditor(
    receipt: ReceiptInput,
    total: String,
    id: String,
    onchange: EventHandler<ReceiptInput>,
) -> Element {
    let store = use_context::<UiState>();
    let current = receipt.clone();
    let update = use_callback(move |change: ReceiptEdit| {
        let mut next = current.clone();
        next.reviewed = false;
        match change {
            ReceiptEdit::Description(index, value) => {
                if let Some(line) = next.lines.get_mut(index) {
                    line.description = value;
                }
            }
            ReceiptEdit::Amount(index, value) => {
                if let Some(line) = next.lines.get_mut(index) {
                    line.amount = value;
                }
            }
            ReceiptEdit::Kind(index, value) => {
                if let Some(line) = next.lines.get_mut(index) {
                    line.kind = ReceiptLineKind::from_code(&value);
                }
            }
            ReceiptEdit::Remove(index) => {
                if index < next.lines.len() {
                    next.lines.remove(index);
                }
            }
            ReceiptEdit::Add => {
                if next.lines.len() < MAX_RECEIPT_LINES {
                    next.lines.push(ReceiptLineInput::default());
                }
            }
            ReceiptEdit::Reviewed(value) => next.reviewed = value,
        }
        onchange.call(next);
    });
    let result = receipt.reconcile(&total);
    let balanced = result.is_ok();
    let status = match result {
        Ok(breakdown) => crate::i18n::text("ยอดตรงกัน {0} บาท", &[breakdown.total().to_string()]),
        Err(_) if receipt.lines.is_empty() => {
            "ยังอ่านรายการสินค้าไม่ได้ เพิ่มรายละเอียดแต่ละรายการจากใบเสร็จ".into()
        }
        Err(_) if total.trim().is_empty() => "กรอกยอดสุทธิในช่องจำนวนเงิน แล้วตรวจผลรวมรายการ".into(),
        Err(error) => error.to_string(),
    };
    rsx! {
        section { class: "receipt-editor", "aria-labelledby": "{id}-lines-title",
            h3 { id: "{id}-lines-title", {crate::i18n::text("รายละเอียดจากใบเสร็จ", &[])} }
            p { class: "field-hint", {crate::i18n::text("ตรวจชื่อและยอดแต่ละบรรทัดกับภาพ ยอดต่อบรรทัดคือยอดรวมของสินค้านั้น ไม่ใช่ราคาต่อชิ้น", &[])} }
            ul { class: "receipt-lines",
                for (index, line) in receipt.lines.iter().enumerate() {
                    li { key: "{index}",
                        div { class: "receipt-line-name",
                            label { r#for: "{id}-name-{index}", {crate::i18n::text("รายการ {0}", &[format!("{}", index + 1)])} }
                            input { id: "{id}-name-{index}", value: line.description.clone(), maxlength: 120, disabled: *store.busy.read(),
                                oninput: move |event| update.call(ReceiptEdit::Description(index, event.value()))
                            }
                        }
                        div { class: "receipt-line-fields",
                            div {
                                label { r#for: "{id}-kind-{index}", {crate::i18n::text("วิธีคิดยอด", &[])} }
                                select { id: "{id}-kind-{index}", value: line.kind.map(ReceiptLineKind::code).unwrap_or_default(), disabled: *store.busy.read(),
                                    onchange: move |event| update.call(ReceiptEdit::Kind(index, event.value())),
                                    option { value: "", disabled: true, selected: line.kind.is_none(), {crate::i18n::text("เลือกวิธีคิดยอด", &[])} }
                                    for kind in ReceiptLineKind::ALL { option { value: kind.code(), selected: line.kind == Some(kind), "{crate::i18n::tr(kind.label())}" } }
                                }
                            }
                            div {
                                label { r#for: "{id}-amount-{index}", {crate::i18n::text("ยอด (บาท)", &[])} }
                                input { id: "{id}-amount-{index}", inputmode: "decimal", value: line.amount.clone(), maxlength: 18, disabled: *store.busy.read(),
                                    oninput: move |event| update.call(ReceiptEdit::Amount(index, event.value()))
                                }
                            }
                            button { class: "icon-button", r#type: "button", "aria-label": crate::i18n::text("ลบรายการใบเสร็จ {0}", &[format!("{}", index + 1)]), disabled: *store.busy.read(),
                                onclick: move |_| update.call(ReceiptEdit::Remove(index)),
                                Icon { name: "close", size: 18 }
                            }
                        }
                    }
                }
            }
            button { class: "soft-button", r#type: "button", disabled: *store.busy.read() || receipt.lines.len() >= MAX_RECEIPT_LINES,
                onclick: move |_| update.call(ReceiptEdit::Add),
                Icon { name: "plus", size: 18 } {crate::i18n::text("เพิ่มบรรทัดใบเสร็จ", &[])}
            }
            p { class: if balanced { "receipt-balance balanced" } else { "receipt-balance mismatch" }, role: "status", "{crate::i18n::tr(&status)}" }
            p { class: "field-hint", {crate::i18n::text("ผลรวม = สินค้า − ส่วนลด + ภาษีและค่าบริการที่บวกเพิ่ม ± ปัดเศษ ภาษีที่รวมในราคาแล้วจะไม่บวกซ้ำ", &[])} }
            label { class: "receipt-reviewed",
                input { r#type: "checkbox", checked: receipt.reviewed, disabled: *store.busy.read() || !balanced,
                    onchange: move |event| update.call(ReceiptEdit::Reviewed(event.checked()))
                }
                span { {crate::i18n::text("ตรวจทุกรายการและยอดสุทธิกับภาพใบเสร็จแล้ว", &[])} }
            }
            p { class: "field-hint", {crate::i18n::text("เมื่อยืนยัน จะบันทึกรายละเอียดเป็น bullet พร้อมยอดแต่ละรายการและยอดสุทธิ", &[])} }
        }
    }
}

enum ReceiptEdit {
    Description(usize, String),
    Amount(usize, String),
    Kind(usize, String),
    Remove(usize),
    Add,
    Reviewed(bool),
}
