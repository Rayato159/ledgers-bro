#![allow(clippy::expect_used)]
use ledger_application::{ReceiptImage, analyze_receipt};
use ledger_infrastructure::{ReceiptImageNormalizer, TesseractOcr};
use std::{path::PathBuf, sync::atomic::AtomicBool};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn fixture(name: &str) -> ReceiptImage {
    ReceiptImage::new(
        std::fs::read(workspace().join("tests/fixtures").join(name)).expect("fixture"),
    )
    .expect("container")
}
#[test]
fn jpeg_exif_rotation_is_applied_and_metadata_removed_before_review() {
    let normalizer = ReceiptImageNormalizer::new(PathBuf::new());
    let image = normalizer
        .normalize(&fixture("receipt-exif6.jpg"), &AtomicBool::new(false))
        .expect("normalize");
    let pixels = image::load_from_memory(image.bytes()).expect("preview");
    assert_eq!((pixels.width(), pixels.height()), (1000, 1200));
    assert!(!image.bytes().windows(6).any(|bytes| bytes == b"Exif\0\0"));
    assert_eq!(image.mime(), "image/jpeg");
}
#[test]
fn normalization_rejects_corrupt_images_and_cancelled_work() {
    let normalizer = ReceiptImageNormalizer::new(PathBuf::new());
    let broken = ReceiptImage::new(b"\x89PNG\r\n\x1a\ninvalid".to_vec()).expect("header");
    assert!(
        normalizer
            .normalize(&broken, &AtomicBool::new(false))
            .is_err()
    );
    assert!(
        normalizer
            .normalize(&fixture("receipt-exif6.jpg"), &AtomicBool::new(true))
            .is_err()
    );
    assert!(
        normalizer
            .normalize(&fixture("receipt-th-en.heic"), &AtomicBool::new(false))
            .expect_err("decoder missing")
            .to_string()
            .contains("HEIC")
    );
}
#[test]
#[ignore = "requires pinned local ImageMagick and Thai/English Tesseract runtime"]
fn heic_jpeg_and_rotated_heic_produce_the_same_total_and_upright_preview() {
    let ocr = TesseractOcr::new(workspace().join(".tools/ocr"));
    for name in [
        "receipt-th-en.png",
        "receipt-th-en.heic",
        "receipt-rotated.heic",
        "receipt-exif6.jpg",
    ] {
        let (preview, text) = ocr
            .scan(&fixture(name), &AtomicBool::new(false))
            .expect(name);
        let pixels = image::load_from_memory(preview.bytes()).expect("preview");
        assert_eq!((pixels.width(), pixels.height()), (1000, 1200), "{name}");
        let analysis =
            analyze_receipt(&text, "2026-09-21".parse().expect("date")).expect("analysis");
        assert_eq!(analysis.totals.len(), 1, "{name}: {text}");
        assert_eq!(analysis.totals[0].amount.minor(), 16050, "{name}: {text}");
        assert!(text.contains("ใบเสร็จ"), "Thai recognition: {name}: {text}");
    }
    let image = ReceiptImageNormalizer::new(workspace().join(".tools/ocr"))
        .normalize(&fixture("receipt-24mp.heic"), &AtomicBool::new(false))
        .expect("24 MP");
    let pixels = image::load_from_memory(image.bytes()).expect("large preview");
    assert_eq!(pixels.height(), 4000);
    assert!(pixels.width() < pixels.height());
}
