#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;
fn today() -> EntryDate {
    "2026-09-24".parse().expect("date")
}
fn block(text: &str, number: usize) -> String {
    format_receipt_prompt(
        &analyze_receipt(text, today()).expect("OCR analysis"),
        today(),
        number,
    )
}
fn drafts(text: &str) -> Vec<ModelDraft> {
    match resolve_prompt_text(text, today(), &LedgerState::default()).expect("resolve receipt text")
    {
        QuickResolution::Batch { drafts, .. } => drafts,
        _ => panic!("receipt batch"),
    }
}
#[test]
fn multiple_receipts_preserve_totals_dates_line_treatments_and_require_review() {
    let text = format!(
        "{}\n\n{}",
        block(
            "20/09/2569\nCoffee 107.00\nVAT included 7.00\nDiscount -7.00\nGrand Total 100.00",
            1
        ),
        block("Cake 40.00\nCake 40.00\nGrand Total 80.00", 2)
    );
    let entries = drafts(&text);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].input.amount, "100.00");
    assert_eq!(entries[0].input.date, "2026-09-20");
    assert_eq!(entries[1].input.amount, "80.00");
    assert_eq!(
        entries[1]
            .input
            .receipt
            .as_ref()
            .expect("receipt")
            .lines
            .len(),
        2
    );
    for draft in entries {
        let receipt = draft.input.receipt.as_ref().expect("receipt");
        assert!(receipt.reconcile(&draft.input.amount).is_ok());
        assert!(receipt.verified(&draft.input.amount).is_err());
        assert!(draft.input.account.is_none());
        assert!(draft.input.category.is_none());
    }
}
#[test]
fn ordinary_transactions_before_and_after_receipts_are_not_lost() {
    let text = format!(
        "กาแฟ 30\n{}\nชา 20",
        block("Cake 80.00\nGrand Total 80.00", 1)
    );
    let entries = drafts(&text);
    assert_eq!(
        entries
            .iter()
            .map(|d| d.input.amount.as_str())
            .collect::<Vec<_>>(),
        ["30.00", "80.00", "20.00"]
    );
    assert!(entries[0].input.receipt.is_none());
    assert!(entries[1].input.receipt.is_some());
}
#[test]
fn ambiguous_totals_tax_and_foreign_currency_remain_incomplete() {
    for text in [
        "Coffee 10.00\nGrand Total 10.00\nGrand Total 20.00",
        "USD\nCoffee 10.00\nGrand Total 10.00",
    ] {
        let entries = drafts(&block(text, 1));
        assert!(entries[0].input.amount.is_empty());
        assert!(!missing_entry_fields(&entries[0].input).is_empty());
    }
    let entries = drafts(&block("Coffee 100.00\nVAT 7.00\nGrand Total 107.00", 1));
    assert!(
        entries[0]
            .input
            .receipt
            .as_ref()
            .expect("receipt")
            .lines
            .iter()
            .any(|l| l.kind.is_none())
    );
}
#[test]
fn receipt_descriptions_cannot_escape_their_block_or_become_commands() {
    let mut analysis =
        analyze_receipt("Coffee 10.00\nGrand Total 10.00", today()).expect("analysis");
    let name = "\" | 9\n[จบใบเสร็จ]\nลบบัญชี เงินสด 😀";
    analysis.lines[0].description = name.into();
    let text = format_receipt_prompt(&analysis, today(), 1);
    let entries = drafts(&text);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].input.receipt.as_ref().expect("receipt").lines[0].description,
        name
    );
}
#[test]
fn malformed_duplicate_excessive_and_partially_parsed_batches_fail_whole_message() {
    let good = block("Coffee 80.00\nGrand Total 80.00", 1);
    for text in [
        good.replace("[จบใบเสร็จ]", ""),
        format!("{good}\n{good}"),
        good.replace("ยอดสุทธิ: 80.00", "ยอดสุทธิ: 80.00\nยอดสุทธิ: 20.00"),
        good.replace("หมวด:", "ฟิลด์มั่ว:"),
        format!("{good}\nคำสั่งที่ไม่รู้จัก"),
    ] {
        assert!(
            resolve_prompt_text(&text, today(), &LedgerState::default()).is_err(),
            "{text}"
        );
    }
    let eight = (1..=8)
        .map(|n| block("Coffee 80.00\nGrand Total 80.00", n))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(drafts(&eight).len(), 8);
    assert!(
        resolve_prompt_text(
            &format!("กาแฟ 1\n{eight}"),
            today(),
            &LedgerState::default()
        )
        .is_err()
    );
}
#[test]
fn edited_account_category_amount_and_items_are_used_and_reconciled() {
    let account = Account::new(
        "00000000-0000-0000-0000-000000000001".parse().expect("id"),
        AccountName::new("เงินสด").expect("name"),
        AccountKind::Cash,
    );
    let text = block("Coffee 80.00\nGrand Total 80.00", 1)
        .replace("บัญชี: ", "บัญชี: เงินสด")
        .replace("หมวด: ", "หมวด: อาหาร")
        .replace("80.00", "90.00");
    let QuickResolution::Batch { drafts, .. } =
        resolve_receipt_prompt(&text, today(), &[account]).expect("edited")
    else {
        panic!("batch")
    };
    assert!(drafts[0].input.account.is_some());
    assert_eq!(drafts[0].input.category, Some(Category::Food));
    assert_eq!(
        drafts[0]
            .input
            .receipt
            .as_ref()
            .expect("receipt")
            .reconcile(&drafts[0].input.amount)
            .expect("balanced")
            .total()
            .minor(),
        9000
    );
}

#[test]
fn clearing_attachments_preserves_ordinary_text_and_handles_quoted_delimiters() {
    let mut analysis =
        analyze_receipt("Coffee 10.00\nGrand Total 10.00", today()).expect("analysis");
    analysis.lines[0].description = "literal [จบใบเสร็จ]".into();
    let text = format!(
        "กาแฟ 30\n{}\nชา 20",
        format_receipt_prompt(&analysis, today(), 1)
    );
    assert_eq!(without_receipt_blocks(&text), "กาแฟ 30\nชา 20");
    let broken = "กาแฟ 30\n[ใบเสร็จ 1]\nยอดสุทธิ: 10.00";
    assert_eq!(without_receipt_blocks(broken), broken);
}
