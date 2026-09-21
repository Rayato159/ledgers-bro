use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::{ReceiptInput, ReceiptLineInput};
use ledger_domain::{MAX_RECEIPT_LINES, ReceiptLineKind};

#[component]
pub fn ReceiptEditor(receipt: ReceiptInput, total: String) -> Element {
    let store = use_context::<UiState>();
    let result = receipt.reconcile(&total);
    let balanced = result.is_ok();
    let status = match result {
        Ok(breakdown) => format!("ยอดตรงกัน {} บาท", breakdown.total()),
        Err(_) if receipt.lines.is_empty() => {
            "ยังอ่านรายการสินค้าไม่ได้ เพิ่มรายละเอียดแต่ละรายการจากใบเสร็จ".into()
        }
        Err(_) if total.trim().is_empty() => "กรอกยอดสุทธิในช่องจำนวนเงิน แล้วตรวจผลรวมรายการ".into(),
        Err(error) => error.to_string(),
    };
    rsx! {
        section { class: "receipt-editor", "aria-labelledby": "receipt-lines-title",
            h3 { id: "receipt-lines-title", "รายละเอียดจากใบเสร็จ" }
            p { class: "field-hint", "ตรวจชื่อและยอดแต่ละบรรทัดกับภาพ ยอดต่อบรรทัดคือยอดรวมของสินค้านั้น ไม่ใช่ราคาต่อชิ้น" }
            ul { class: "receipt-lines",
                for (index, line) in receipt.lines.iter().enumerate() {
                    li { key: "{index}",
                        div { class: "receipt-line-name",
                            label { r#for: "receipt-name-{index}", "รายการ {index + 1}" }
                            input { id: "receipt-name-{index}", value: line.description.clone(), maxlength: 120, disabled: *store.busy.read(),
                                oninput: move |event| store.update_receipt(|receipt| { if let Some(line) = receipt.lines.get_mut(index) { line.description = event.value(); } })
                            }
                        }
                        div { class: "receipt-line-fields",
                            div {
                                label { r#for: "receipt-kind-{index}", "วิธีคิดยอด" }
                                select { id: "receipt-kind-{index}", value: line.kind.map(ReceiptLineKind::code).unwrap_or_default(), disabled: *store.busy.read(),
                                    onchange: move |event| store.update_receipt(|receipt| { if let Some(line) = receipt.lines.get_mut(index) { line.kind = ReceiptLineKind::from_code(&event.value()); } }),
                                    option { value: "", disabled: true, "เลือกวิธีคิดยอด" }
                                    for kind in ReceiptLineKind::ALL { option { value: kind.code(), "{kind.label()}" } }
                                }
                            }
                            div {
                                label { r#for: "receipt-amount-{index}", "ยอด (บาท)" }
                                input { id: "receipt-amount-{index}", inputmode: "decimal", value: line.amount.clone(), maxlength: 18, disabled: *store.busy.read(),
                                    oninput: move |event| store.update_receipt(|receipt| { if let Some(line) = receipt.lines.get_mut(index) { line.amount = event.value(); } })
                                }
                            }
                            button { class: "icon-button", r#type: "button", "aria-label": "ลบรายการใบเสร็จ {index + 1}", disabled: *store.busy.read(),
                                onclick: move |_| store.update_receipt(|receipt| { if index < receipt.lines.len() { receipt.lines.remove(index); } }),
                                Icon { name: "close", size: 18 }
                            }
                        }
                    }
                }
            }
            button { class: "soft-button", r#type: "button", disabled: *store.busy.read() || receipt.lines.len() >= MAX_RECEIPT_LINES,
                onclick: move |_| store.update_receipt(|receipt| { if receipt.lines.len() < MAX_RECEIPT_LINES { receipt.lines.push(ReceiptLineInput::default()); } }),
                Icon { name: "plus", size: 18 } "เพิ่มบรรทัดใบเสร็จ"
            }
            p { class: if balanced { "receipt-balance balanced" } else { "receipt-balance mismatch" }, role: "status", "{status}" }
            p { class: "field-hint", "ผลรวม = สินค้า − ส่วนลด + ภาษีและค่าบริการที่บวกเพิ่ม ± ปัดเศษ ภาษีที่รวมในราคาแล้วจะไม่บวกซ้ำ" }
            label { class: "receipt-reviewed",
                input { r#type: "checkbox", checked: receipt.reviewed, disabled: *store.busy.read() || !balanced,
                    onchange: move |event| store.update_entry(|input| { if let Some(receipt) = &mut input.receipt { receipt.reviewed = event.checked(); } })
                }
                span { "ตรวจทุกรายการและยอดสุทธิกับภาพใบเสร็จแล้ว" }
            }
            p { class: "field-hint", "เมื่อยืนยัน จะบันทึกรายละเอียดเป็น bullet พร้อมยอดแต่ละรายการและยอดสุทธิ" }
        }
    }
}
