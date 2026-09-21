#![allow(clippy::expect_used)]
use ledger_application::{ReceiptImage, ReceiptOcr, analyze_receipt};
use ledger_infrastructure::TesseractOcr;
use std::{path::PathBuf, sync::atomic::AtomicBool};

#[test]
fn missing_runtime_is_a_recoverable_error() {
    let image = ReceiptImage::new(b"\x89PNG\r\n\x1a\n".to_vec()).expect("signature");
    let directory = tempfile::tempdir().expect("temp");
    assert!(
        TesseractOcr::new(directory.path().to_owned())
            .recognize(&image, &AtomicBool::new(false))
            .is_err()
    );
}

#[test]
fn cancellation_is_observed_before_launching_native_code() {
    let image = ReceiptImage::new(b"\x89PNG\r\n\x1a\n".to_vec()).expect("signature");
    let error = TesseractOcr::new(PathBuf::new())
        .recognize(&image, &AtomicBool::new(true))
        .expect_err("cancelled");
    assert!(error.to_string().contains("ยกเลิก"));
}

#[test]
#[ignore = "requires the locally installed OCR runtime and synthetic receipt fixture"]
fn local_ocr_reads_a_real_image_and_produces_a_reviewable_draft() {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(workspace.join("tests/fixtures/receipt-th-en.png")).expect("fixture");
    let image = ReceiptImage::new(bytes).expect("image");
    let ocr = TesseractOcr::new(workspace.join(".tools/ocr"));
    let text = ocr
        .recognize(&image, &AtomicBool::new(false))
        .expect("local OCR");
    let today = "2026-09-20".parse().expect("date");
    let result = analyze_receipt(&text, today).expect("analysis");
    assert!(
        text.contains("ใบเสร็จ"),
        "Thai model must read Thai text: {text}"
    );
    assert_eq!(result.draft(today).amount, "160.50", "{text}");
    assert_eq!(result.draft(today).date, "2026-09-20", "{text}");
    let mut receipt = result.draft(today).receipt.expect("receipt lines");
    assert_eq!(receipt.lines.len(), 3, "two items and VAT: {text}");
    assert_eq!(receipt.lines[0].amount, "80.00", "{text}");
    assert_eq!(receipt.lines[1].amount, "70.00", "{text}");
    assert!(
        receipt.lines[2].kind.is_none(),
        "VAT treatment requires a choice"
    );
    receipt.lines[2].kind = Some(ledger_domain::ReceiptLineKind::AddedTax);
    receipt.reviewed = true;
    assert_eq!(
        receipt
            .verified("160.50")
            .expect("real OCR lines balance")
            .total()
            .minor(),
        16_050
    );
    let invalid = ReceiptImage::new(b"\x89PNG\r\n\x1a\ninvalid".to_vec()).expect("signature");
    assert!(ocr.recognize(&invalid, &AtomicBool::new(false)).is_err());
}
