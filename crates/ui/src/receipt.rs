use crate::{components::*, state::*};
use base64::Engine;
use dioxus::prelude::*;
use ledger_application::{ReceiptAnalysis, ReceiptImage};

#[derive(Clone, PartialEq)]
pub struct ReceiptReview {
    pub preview: String,
    pub byte_len: u64,
    pub analysis: ReceiptAnalysis,
}
impl ReceiptReview {
    pub fn new(image: ReceiptImage, analysis: ReceiptAnalysis) -> Self {
        let preview = format!(
            "data:{};base64,{}",
            image.mime(),
            base64::engine::general_purpose::STANDARD.encode(image.bytes())
        );
        Self {
            preview,
            byte_len: image.bytes().len() as u64,
            analysis,
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct ReceiptAttachment {
    pub name: String,
    pub review: Result<ReceiptReview, String>,
}
#[derive(Clone, Copy, PartialEq)]
pub enum ReceiptDestination {
    Prompt,
    Manual,
}

#[component]
pub fn ReceiptUpload(
    destination: ReceiptDestination,
    #[props(default)] disabled: bool,
    #[props(default)] compact: bool,
) -> Element {
    let store = use_context::<UiState>();
    let host = use_context::<crate::HostInfo>();
    let unsupported_currency = crate::i18n::currency() != ledger_domain::Currency::Thb;
    let disabled =
        disabled || *store.busy.read() || !host.receipt_ocr_available || unsupported_currency;
    let native = store.gateway.read().0.uses_native_receipt_picker();
    let id = if destination == ReceiptDestination::Prompt {
        "prompt-receipts"
    } else {
        "manual-receipts"
    };
    rsx! {
        if unsupported_currency { span { class: "field-hint", "Receipt OCR: THB only" } }
        if native {
            button { class: "receipt-upload-inline soft-button", r#type: "button", disabled, onclick: move |_| store.pick_receipts(destination),
                Icon { name: "camera", size: 22 } span { class: if compact { "sr-only" } else { "" }, {crate::i18n::text("เพิ่มใบเสร็จ", &[])} }
            }
        } else {
            label { class: if disabled { "receipt-upload-inline soft-button disabled" } else { "receipt-upload-inline soft-button" },
                input { id, name: "receipts", class: "receipt-file-input", r#type: "file", multiple: true,
                    accept: ".jpg,.jpeg,.png,.heic,.heif,image/png,image/jpeg,image/heic,image/heif", disabled,
                    "aria-label": crate::i18n::text("อัปโหลดรูปใบเสร็จ เลือกได้หลายรูป", &[]),
                    onclick: move |_| { let _ = document::eval(&format!("document.getElementById('{id}').value = ''")); },
                    onchange: move |event| store.scan_receipts(event.files(), destination),
                }
                Icon { name: "camera", size: 22 } span { class: if compact { "sr-only" } else { "" }, {crate::i18n::text("เพิ่มใบเสร็จ", &[])} }
            }
        }
    }
}

#[component]
pub fn ReceiptAttachments() -> Element {
    let store = use_context::<UiState>();
    let attachments = store.receipts.read().clone();
    rsx! {
        if store.scan_cancel.read().is_some() {
            div { class: "scan-progress", role: "status", span { class: "scan-pulse" } "{store.scan_progress}"
                button { r#type: "button", class: "text-button", onclick: move |_| store.cancel_scan(), {crate::i18n::text("ยกเลิกการอ่าน", &[])} }
            }
        }
        if !attachments.is_empty() {
            div { class: "receipt-attachments",
                button { r#type: "button", class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.clear_receipts(), {crate::i18n::text("ล้างรูปและรายการจากใบเสร็จชุดนี้", &[])} }
                for (index, attachment) in attachments.iter().enumerate() {
                    details { class: "receipt-attachment",
                        summary {
                            if let Ok(review) = &attachment.review { img { src: review.preview.clone(), alt: "", class: "receipt-thumbnail" } }
                            span { strong { {crate::i18n::text("ใบเสร็จ {0}", &[format!("{}", index + 1)])} } span { "{attachment.name}" } }
                            if attachment.review.is_err() { span { class: "inline-warning", {crate::i18n::text("อ่านไม่สำเร็จ", &[])} } } else { span { {crate::i18n::text("ดูภาพ / OCR", &[])} } }
                        }
                        match &attachment.review {
                            Err(error) => rsx! { p { class: "inline-warning", role: "alert", {crate::i18n::text("{0} · ลองเลือกรูปนี้ใหม่", std::slice::from_ref(error))} } },
                            Ok(review) => rsx! {
                                img { class: "receipt-preview", src: review.preview.clone(), alt: crate::i18n::text("ภาพใบเสร็จ {0} สำหรับตรวจรายการ", &[format!("{}", index + 1)]) }
                                if review.analysis.foreign_currency { p { class: "inline-warning", {crate::i18n::text("พบสกุลเงินอื่น กรอกยอดและรายละเอียดที่จ่ายจริงเป็นบาท แอปไม่แปลงค่าเงินให้อัตโนมัติ", &[])} } }
                                if review.analysis.totals.len() != 1 { p { class: "inline-warning", {crate::i18n::text("ยอดสุทธิไม่ชัด กรุณาเติมยอดที่จ่ายจริง", &[])} } }
                                if review.analysis.dates.len() != 1 { p { class: "field-hint", {crate::i18n::text("วันที่ไม่ชัด เริ่มต้นเป็นวันนี้ กรุณาตรวจวันที่กับภาพ", &[])} } }
                                for candidate in &review.analysis.totals { p { class: "field-hint", {crate::i18n::text("ยอดที่พบ: {0} บาท · {1}", &[format!("{}", candidate.amount), candidate.evidence.to_string()])} } }
                                pre { class: "ocr-text", "{review.analysis.text}" }
                            },
                        }
                    }
                }
            }
        }
    }
}
