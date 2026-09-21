#![allow(clippy::expect_used)]
use ledger_application::*;
use ledger_domain::*;

fn analyze(text: &str) -> ReceiptAnalysis {
    analyze_receipt(text, "2026-09-20".parse().expect("date")).expect("analysis")
}

#[test]
fn item_lines_preserve_duplicates_and_quantity_text_while_summary_lines_are_excluded() {
    let analysis = analyze(
        "TEST CAFE\n20/09/2569\nCoffee 2 x 40.00 80.00\nCake 70.00\nCake 70.00\nSubtotal 220.00\nGrand Total 220.00\nCash 300.00\nChange 80.00",
    );
    assert_eq!(analysis.lines.len(), 3);
    assert_eq!(analysis.lines[0].description, "Coffee 2 x 40.00");
    assert_eq!(analysis.lines[0].amount, "80.00");
    assert_eq!(analysis.lines[1], analysis.lines[2]);
    let draft = analysis.draft("2026-09-20".parse().expect("date"));
    let receipt = draft.receipt.expect("receipt draft");
    assert_eq!(
        receipt
            .reconcile(&draft.amount)
            .expect("balanced")
            .total()
            .minor(),
        22_000
    );
    assert!(
        receipt.verified(&draft.amount).is_err(),
        "OCR is never automatically marked reviewed"
    );
}

#[test]
fn vat_requires_explicit_treatment_then_can_be_reconciled_and_reviewed() {
    let analysis =
        analyze("Coffee 80.00\nLunch 70.00\nSubtotal 150.00\nVAT 7% 10.50\nGrand Total 160.50");
    assert_eq!(analysis.lines.len(), 3);
    assert_eq!(analysis.lines[2].kind, None);
    let mut receipt = ReceiptInput {
        lines: analysis.lines,
        reviewed: false,
    };
    assert!(receipt.reconcile("160.50").is_err());
    receipt.lines[2].kind = Some(ReceiptLineKind::AddedTax);
    assert!(receipt.reconcile("160.50").is_ok());
    assert!(receipt.verified("160.50").is_err());
    receipt.reviewed = true;
    assert!(receipt.verified("160.50").is_ok());
    assert!(receipt.verified("160.51").is_err());
}

#[test]
fn included_tax_discount_and_rounding_are_not_treated_as_goods() {
    let analysis =
        analyze("กาแฟ ๑๐๗.๐๐\nVAT included ๗.๐๐\nส่วนลด -๗.๐๐\nปัดเศษ -๐.๐๑\nยอดสุทธิ ๙๙.๙๙ บาท");
    assert_eq!(analysis.lines.len(), 4);
    let receipt = ReceiptInput {
        lines: analysis.lines,
        reviewed: true,
    };
    let reconciled = receipt.verified("99.99").expect("balanced");
    assert_eq!(reconciled.total().minor(), 9999);
}

#[test]
fn unread_items_or_wrong_totals_cannot_be_replaced_with_a_fabricated_balancing_line() {
    for text in ["Grand Total 80.00", "Coffee 70.00\nGrand Total 80.00"] {
        let analysis = analyze(text);
        let receipt = ReceiptInput {
            lines: analysis.lines,
            reviewed: true,
        };
        assert!(receipt.verified("80.00").is_err());
    }
}

#[test]
fn foreign_line_amounts_require_explicit_thb_input_and_parser_is_bounded() {
    let analysis = analyze("USD\nCoffee 2.50\nTotal 2.50");
    assert!(analysis.lines[0].amount.is_empty());
    assert!(
        analysis
            .draft("2026-09-20".parse().expect("date"))
            .amount
            .is_empty()
    );
    assert!(
        analyze_receipt(
            &"Coffee 1.00\n".repeat(101),
            "2026-09-20".parse().expect("date")
        )
        .is_err()
    );
}
