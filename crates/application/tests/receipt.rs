#![allow(clippy::expect_used)]
use ledger_application::{MAX_RECEIPT_BYTES, ReceiptImage, analyze_receipt};

fn analyze(text: &str) -> ledger_application::ReceiptAnalysis {
    analyze_receipt(text, "2026-09-20".parse().expect("date")).expect("analysis")
}

#[test]
fn net_total_excludes_tax_subtotal_cash_and_change() {
    let result = analyze(
        "TEST CAFE\n20/09/2569\nSubtotal 150.00\nVAT 10.50\nGrand Total 160.50\nCash 200.00\nChange 39.50",
    );
    assert_eq!(result.totals.len(), 1);
    assert_eq!(result.totals[0].amount.to_string(), "160.50");
    let draft = result.draft("2026-09-20".parse().expect("date"));
    assert_eq!(draft.date, "2026-09-20");
    assert_eq!(draft.amount, "160.50");
    assert!(draft.account.is_none());
    assert!(draft.category.is_none());
    assert!(!draft.is_complete());
}

#[test]
fn ambiguous_totals_require_a_choice_and_dates_default_to_today() {
    let result = analyze("19/09/2026\n20/09/2026\nTOTAL 99.00\nยอดสุทธิ 199.00");
    let draft = result.draft("2026-09-20".parse().expect("date"));
    assert_eq!(result.totals.len(), 2);
    assert_eq!(result.dates.len(), 2);
    assert!(draft.amount.is_empty());
    assert_eq!(draft.date, "2026-09-20");
}

#[test]
fn thai_numerals_and_thousands_separators_preserve_satang() {
    let result = analyze("๒๐/๐๙/๒๕๖๙\nยอดสุทธิ ๑,๒๓๔.๕๖ บาท");
    assert_eq!(result.totals[0].amount.minor(), 123456);
    assert_eq!(result.dates[0].to_string(), "2026-09-20");
}

#[test]
fn foreign_currency_never_becomes_thb_automatically() {
    for currency in ["USD", "$", "EUR", "€", "GBP", "JPY", "SGD"] {
        let result = analyze(&format!("TOTAL {currency} 10.99"));
        assert!(result.foreign_currency);
        assert!(
            result
                .draft("2026-09-20".parse().expect("date"))
                .amount
                .is_empty()
        );
    }
}

#[test]
fn malformed_amounts_and_unlabelled_prices_are_not_guessed() {
    for text in [
        "Total 12,34",
        "Total 12.3.4",
        "Total -80.00",
        "Total 0.00",
        "Total 9000000000000000.00",
        "Coffee 80.00\n200.00",
        "SUB TOTAL 80.00",
        "Total VAT 5.60",
        "Total 12.3,4",
    ] {
        assert!(analyze(text).totals.is_empty(), "{text}");
    }
}

#[test]
fn missing_invalid_future_and_two_digit_dates_default_to_today() {
    for text in [
        "Total 80.00",
        "31/02/2026",
        "21/09/2026",
        "20/09/26",
        "9999-01-01",
    ] {
        assert!(analyze(text).dates.is_empty());
        assert_eq!(
            analyze(text)
                .draft("2026-09-20".parse().expect("date"))
                .date,
            "2026-09-20"
        );
    }
}

#[test]
fn explicit_receipt_date_is_not_replaced_by_today() {
    assert_eq!(
        analyze("18/09/2569\nTotal 80.00")
            .draft("2026-09-20".parse().expect("date"))
            .date,
        "2026-09-18"
    );
}

#[test]
fn repeated_totals_are_deduplicated_and_output_is_bounded() {
    assert_eq!(analyze("TOTAL 80.00\nยอดสุทธิ 80.00").totals.len(), 1);
    assert!(analyze_receipt(&"a".repeat(65537), "2026-09-20".parse().expect("date")).is_err());
}

#[test]
fn receipt_image_rejects_non_images_and_excessive_files() {
    assert!(ReceiptImage::new(b"<svg onload='anything'>".to_vec()).is_err());
    assert!(ReceiptImage::new(vec![0; MAX_RECEIPT_BYTES + 1]).is_err());
}

#[test]
fn receipt_accepts_heif_container_brands_without_trusting_a_filename() {
    let mut header = vec![0, 0, 0, 24];
    header.extend_from_slice(b"ftypmif1\0\0\0\0heicmif1");
    assert_eq!(
        ReceiptImage::new(header.clone()).expect("HEIF").mime(),
        "image/heif"
    );
    header[16..20].copy_from_slice(b"avif");
    assert!(ReceiptImage::new(header.clone()).is_err());
    header[16..20].copy_from_slice(b"hevc");
    assert!(ReceiptImage::new(header.clone()).is_err());
    header[16..20].copy_from_slice(b"heic");
    header[3] = 255;
    assert!(ReceiptImage::new(header).is_err());
    assert!(ReceiptImage::new(b"\0\0\0\x18ftypheic".to_vec()).is_err());
}
