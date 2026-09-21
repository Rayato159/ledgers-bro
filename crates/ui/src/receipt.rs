use crate::{artwork::*, components::*, state::*};
use base64::Engine;
use dioxus::prelude::*;
use ledger_application::{ReceiptAnalysis, ReceiptImage};

#[derive(Clone, PartialEq)]
pub struct ReceiptReview {
    pub preview: String,
    pub analysis: ReceiptAnalysis,
}
impl ReceiptReview {
    pub fn new(image: ReceiptImage, analysis: ReceiptAnalysis) -> Self {
        let preview = format!(
            "data:{};base64,{}",
            image.mime(),
            base64::engine::general_purpose::STANDARD.encode(image.bytes())
        );
        Self { preview, analysis }
    }
}

#[component]
pub fn ReceiptScanner() -> Element {
    let store = use_context::<UiState>();
    let host = use_context::<crate::HostInfo>();
    let scanning = store.scan_cancel.read().is_some();
    let receipt = store.receipt.read().clone();
    let native_picker = store.gateway.read().0.uses_native_receipt_picker();
    rsx! {
        section { class: "receipt-scanner", "aria-labelledby": "receipt-title",
            div { class: "scanner-heading", ArtIcon { name: "receipt", size: 52 }
                div { h2 { id: "receipt-title", "สแกนใบเสร็จ" }
                    if host.receipt_ocr_available { p { "เลือกรูป แล้วช่วยกันตรวจรายการ" } }
                }
                span { class: "local-badge", Icon { name: "device", size: 12 }
                    if host.receipt_ocr_available { "ในเครื่อง" } else { "ยังไม่พร้อม" }
                }
            }
            if !host.receipt_ocr_available {
                p { class: "field-hint", "รุ่นทดลอง Android ยังไม่รองรับการอ่านใบเสร็จ ใช้แบบฟอร์มบันทึกรายการในหน้านี้ได้" }
            } else if scanning {
                div { class: "scan-progress", role: "status", span { class: "scan-pulse" } "กำลังอ่านข้อความในใบเสร็จ…" }
                button { class: "text-button", onclick: move |_| store.cancel_scan(), "ยกเลิกการอ่าน" }
            } else if native_picker {
                button { class: "receipt-upload", disabled: *store.busy.read(), onclick: move |_| store.pick_receipt(),
                    Icon { name: "camera", size: 22 } span { "เลือกรูปใบเสร็จ" }
                    small { "JPG / PNG / HEIC / HEIF · ไม่เกิน 32 MB" }
                }
            } else {
                label { class: if *store.busy.read() { "receipt-upload disabled" } else { "receipt-upload" },
                    input { id: "receipt-file", name: "receipt", class: "receipt-file-input", r#type: "file", accept: ".jpg,.jpeg,.png,.heic,.heif,image/png,image/jpeg,image/heic,image/heif", disabled: *store.busy.read(),
                        "aria-label": "เลือกรูปใบเสร็จ",
                        onclick: move |_| { let _ = document::eval("document.getElementById('receipt-file').value = ''"); },
                        onchange: move |event| { if let Some(file) = event.files().into_iter().next() { store.scan_receipt(file); } }
                    }
                    Icon { name: "camera", size: 22 } span { if receipt.is_some() { "เปลี่ยนรูปใบเสร็จ" } else { "เลือกรูปใบเสร็จ" } }
                    small { "JPG / PNG / HEIC / HEIF · ไม่เกิน 32 MB" }
                }
            }
            if host.receipt_ocr_available {
                p { class: "scanner-privacy", "อ่านภาพบนเครื่องนี้ ไม่ส่งขึ้น Cloud • ภาพและข้อความใช้ชั่วคราว ไม่ได้แนบเก็บกับรายการ" }
            }
            if let Some(receipt) = receipt {
                ReceiptResult { receipt }
            }
        }
    }
}

#[component]
fn ReceiptResult(receipt: ReceiptReview) -> Element {
    let store = use_context::<UiState>();
    let analysis = receipt.analysis;
    rsx! {
        div { class: "receipt-result",
            img { class: "receipt-preview", src: receipt.preview, alt: "ใบเสร็จที่เลือก ตรวจเทียบกับยอดก่อนบันทึก" }
            div { class: "receipt-findings", strong { "ตรวจจากภาพอีกครั้ง" }
                if analysis.foreign_currency {
                    p { class: "inline-warning", "พบสกุลเงินต่างประเทศ กรอกยอดที่จ่ายจริงเป็นบาทในช่องจำนวนเงิน แอปยังไม่แปลงค่าเงินให้อัตโนมัติ" }
                } else if analysis.totals.is_empty() {
                    p { class: "field-hint", "ยังแยกยอดสุทธิไม่ได้ กรอกยอดจากใบเสร็จในช่องจำนวนเงิน" }
                } else {
                    p { class: "field-hint", if analysis.totals.len() > 1 { "พบหลายยอด เลือกยอดที่จ่ายจริง" } else { "ยอดที่อ่านได้ แก้ไขได้ในช่องจำนวนเงิน" } }
                    div { class: "receipt-choices",
                        for candidate in analysis.totals {
                            { let amount = candidate.amount.to_string(); rsx! {
                                button { class: "receipt-choice", disabled: *store.busy.read(), onclick: move |_| store.update_entry(|input| input.amount = amount.clone()),
                                    strong { "฿{money_label(candidate.amount)}" } small { "{candidate.evidence}" }
                                }
                            } }
                        }
                    }
                }
                if analysis.dates.is_empty() { p { class: "field-hint", "ยังอ่านวันที่ไม่ชัด เริ่มต้นเป็นวันนี้ แก้วันที่ให้ตรงกับใบเสร็จได้" } }
                if analysis.dates.len() > 1 {
                    p { class: "field-hint", "พบหลายวันที่ เริ่มต้นเป็นวันนี้ เลือกวันที่จากใบเสร็จด้านล่างหรือแก้ในช่องวันที่" }
                    div { class: "choice-chips", for date in analysis.dates { button { class: "choice-chip", disabled: *store.busy.read(), onclick: move |_| store.update_entry(|input| input.date = date.to_string()), "{date}" } } }
                }
            }
        }
        details { class: "ocr-text", summary { "ดูข้อความที่อ่านได้" } pre { "{analysis.text}" } }
    }
}
