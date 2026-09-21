#![allow(clippy::expect_used)]
use ledger_domain::*;

fn line(name: &str, amount: &str, kind: ReceiptLineKind) -> ReceiptLine {
    ReceiptLine::new(name, amount.parse().expect("money"), kind).expect("line")
}
fn total(value: &str) -> PositiveMoney {
    PositiveMoney::new(value.parse().expect("money")).expect("positive")
}

#[test]
fn exact_satang_reconciliation_accounts_for_adjustments_without_double_counting_tax() {
    let receipt = ReceiptBreakdown::new(
        vec![
            line("กาแฟ", "20.10", ReceiptLineKind::Item),
            line("ขนม", "10.20", ReceiptLineKind::Item),
            line("ส่วนลด", "3", ReceiptLineKind::Discount),
            line("ค่าบริการ", "3", ReceiptLineKind::ServiceCharge),
            line("VAT เพิ่ม", "1.89", ReceiptLineKind::AddedTax),
            line("VAT ในราคา", "1", ReceiptLineKind::IncludedCharge),
            line("ปัดเศษ", "-0.01", ReceiptLineKind::Rounding),
        ],
        total("32.18"),
    )
    .expect("balanced");
    assert_eq!(receipt.total().minor(), 3218);
    let note = receipt.note("มื้อเที่ยง").expect("note");
    assert!(note.as_str().contains("• กาแฟ — 20.10 บาท\n"));
    assert!(note.as_str().contains("• ส่วนลด — -3.00 บาท\n"));
    assert!(
        note.as_str()
            .contains("• VAT ในราคา — 1.00 บาท (รวมในราคาแล้ว ไม่บวกซ้ำ)\n")
    );
    assert!(
        note.as_str()
            .ends_with("ยอดสุทธิ 32.18 บาท\n\nหมายเหตุ: มื้อเที่ยง")
    );
}

#[test]
fn even_one_satang_mismatch_is_rejected() {
    let error = ReceiptBreakdown::new(
        vec![
            line("กาแฟ", "80", ReceiptLineKind::Item),
            line("ข้าว", "70", ReceiptLineKind::Item),
        ],
        total("150.01"),
    )
    .expect_err("must not round a mismatch away");
    assert_eq!(
        error,
        DomainError::ReceiptTotalMismatch {
            calculated: "150".parse().expect("money"),
            total: "150.01".parse().expect("money"),
        }
    );
}

#[test]
fn incomplete_invalid_and_overflowing_lines_cannot_form_a_receipt() {
    assert!(ReceiptLine::new("", Money::ZERO, ReceiptLineKind::Item).is_err());
    assert!(ReceiptLine::new("ชื่อ\nปลอม", Money::ZERO, ReceiptLineKind::Item).is_err());
    assert!(ReceiptLine::new("สินค้า", "-1".parse().expect("money"), ReceiptLineKind::Item).is_err());
    assert!(ReceiptBreakdown::new(vec![], total("1")).is_err());
    assert!(
        ReceiptBreakdown::new(
            vec![line("ภาษี", "1", ReceiptLineKind::AddedTax)],
            total("1")
        )
        .is_err()
    );
    assert!(
        ReceiptBreakdown::new(
            vec![line("สินค้า", "1", ReceiptLineKind::Item); MAX_RECEIPT_LINES + 1],
            total("101")
        )
        .is_err()
    );
    assert!(
        ReceiptBreakdown::new(
            vec![
                line("สินค้า", "90000000000", ReceiptLineKind::Item),
                line("เพิ่ม", "1", ReceiptLineKind::ServiceCharge),
            ],
            total("90000000000")
        )
        .is_err()
    );
    assert!(
        ReceiptBreakdown::new(
            vec![
                line("สินค้า", "80", ReceiptLineKind::Item),
                line("ภาษีรวม", "81", ReceiptLineKind::IncludedCharge),
            ],
            total("80")
        )
        .is_err()
    );
}

#[test]
fn complete_bullet_details_are_not_silently_truncated_to_the_old_note_limit() {
    let receipt = ReceiptBreakdown::new(
        (1..=100)
            .map(|i| {
                line(
                    &format!("สินค้า {i} {}", "ก".repeat(90)),
                    "1",
                    ReceiptLineKind::Item,
                )
            })
            .collect(),
        total("100"),
    )
    .expect("large receipt");
    let note = receipt.note("").expect("all lines fit the bounded note");
    assert_eq!(
        note.as_str()
            .lines()
            .filter(|line| line.starts_with("• "))
            .count(),
        100
    );
    assert!(note.as_str().chars().count() > 500);
    assert!(note.as_str().ends_with("ยอดสุทธิ 100.00 บาท"));
    assert!(Note::new(&"ก".repeat(Note::MAX_CHARS + 1)).is_err());
}
