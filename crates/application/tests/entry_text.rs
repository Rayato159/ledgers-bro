#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;

fn accounts() -> Vec<Account> {
    ["เงินสด", "กรุงไทย", "เงินเก็บ และ สำรอง", "กรุงไทย2"]
        .iter()
        .enumerate()
        .map(|(i, name)| {
            Account::new(
                format!("00000000-0000-0000-0000-{:012}", i + 1)
                    .parse()
                    .expect("id"),
                AccountName::new(name).expect("name"),
                AccountKind::Cash,
            )
        })
        .collect()
}
fn resolve(text: &str) -> Result<QuickResolution, AppError> {
    resolve_entry_text(text, "2026-01-01".parse().expect("date"), &accounts())
}
fn batch(text: &str) -> Vec<ModelDraft> {
    match resolve(text).expect(text) {
        QuickResolution::Batch { drafts, .. } => drafts,
        _ => panic!("expected batch: {text}"),
    }
}
#[test]
fn users_exact_example_preserves_two_amounts_notes_and_ordered_accounts() {
    let drafts =
        batch("ซื้อไก่ทอดไป 30 บาท และได้เงินจาก Facebook 400 บาท บันทึกลงเงินสด และ กรุงไทยตามลำดับ");
    assert_eq!(drafts.len(), 2);
    for (i, kind, amount, note) in [
        (0, TransactionKind::Expense, "30.00", "ไก่ทอด"),
        (1, TransactionKind::Income, "400.00", "Facebook"),
    ] {
        let input = &drafts[i].input;
        assert_eq!(input.kind, kind);
        assert_eq!(input.amount, amount);
        assert_eq!(input.account, Some(accounts()[i].id()));
        assert_eq!(input.note, note);
        assert_eq!(input.date, "2026-01-01");
        assert_eq!(missing_entry_fields(input), ["หมวดหมู่"]);
    }
}
#[test]
fn realistic_clauses_support_dates_decimals_thousands_and_explicit_categories() {
    for (source, amount, kind, category, account, date) in [
        (
            "เมื่อวานซื้อข้าว 65.50 บาท จาก เงินสด หมวด อาหาร",
            "65.50",
            TransactionKind::Expense,
            Category::Food,
            0,
            "2025-12-31",
        ),
        (
            "ได้รับเงินเดือน 30,000 บาท เข้า กรุงไทย หมวด เงินเดือน",
            "30000.00",
            TransactionKind::Income,
            Category::Salary,
            1,
            "2026-01-01",
        ),
        (
            "จ่ายค่าเช่า 8,500.25 บาท จาก กรุงไทย หมวด ค่าเช่า วันนี้",
            "8500.25",
            TransactionKind::Expense,
            Category::Rent,
            1,
            "2026-01-01",
        ),
        (
            "ซื้อข้าว50บาทจากกรุงไทย2หมวดอาหาร",
            "50.00",
            TransactionKind::Expense,
            Category::Food,
            3,
            "2026-01-01",
        ),
    ] {
        let drafts = batch(source);
        assert_eq!(drafts.len(), 1);
        let input = &drafts[0].input;
        assert_eq!(input.amount, amount, "{source}");
        assert_eq!(input.kind, kind);
        assert_eq!(input.category, Some(category));
        assert_eq!(input.account, Some(accounts()[account].id()));
        assert_eq!(input.date, date);
        assert!(missing_entry_fields(input).is_empty());
    }
}
#[test]
fn newline_and_quoted_names_and_notes_do_not_lose_clauses() {
    let drafts = batch(
        "จ่าย 50 จาก \"เงินเก็บ และ สำรอง\" หมวด อาหาร โน้ต \"ข้าวและน้ำ\"\nรับ 100 เข้า กรุงไทย หมวด ฟรีแลนซ์",
    );
    assert_eq!(drafts.len(), 2);
    assert_eq!(drafts[0].input.account, Some(accounts()[2].id()));
    assert_eq!(drafts[0].input.note, "ข้าวและน้ำ");
    let drafts = batch("ซื้อข้าว50บาท และซื้อชา20บาท บันทึกลง \"เงินเก็บ และ สำรอง\" และ เงินสด ตามลำดับ");
    assert_eq!(drafts[0].input.account, Some(accounts()[2].id()));
    assert_eq!(drafts[1].input.account, Some(accounts()[0].id()));
}
#[test]
fn unclear_account_mapping_requires_a_choice_for_every_item() {
    for source in [
        "ซื้อข้าว50บาท และซื้อชา20บาท บันทึกลง เงินสด และ กรุงไทย",
        "ซื้อข้าว50บาท และซื้อชา20บาท บันทึกลง เงินสด ตามลำดับ",
        "ซื้อข้าว50บาท และซื้อชา20บาท บันทึกลง เงินสด และ กรุงไทย และ กรุงไทย2 ตามลำดับ",
    ] {
        let drafts = batch(source);
        assert_eq!(drafts.len(), 2);
        assert!(
            drafts
                .iter()
                .all(|d| d.input.account.is_none() && !d.guidance.is_empty())
        );
    }
    let drafts = batch("ซื้อข้าว50บาท จากเงินสด บันทึกลงกรุงไทย");
    assert!(drafts[0].input.account.is_none());
    assert!(drafts[0].guidance.contains("ขัด"));
}
#[test]
fn missing_fields_and_unknown_accounts_are_actionable_not_invented() {
    let drafts = batch("ซื้อไก่ทอด");
    assert_eq!(
        missing_entry_fields(&drafts[0].input),
        ["ยอดเงินที่มากกว่า 0", "บัญชีที่ใช้รับหรือจ่าย", "หมวดหมู่"]
    );
    let drafts = batch("ซื้อไก่ทอด30บาท จาก กสิกร หมวด อาหาน");
    assert!(drafts[0].guidance.contains("กสิกร"));
    assert_eq!(
        missing_entry_fields(&drafts[0].input),
        ["บัญชีที่ใช้รับหรือจ่าย", "หมวดหมู่"]
    );
    let drafts = batch("ซื้อไก่ทอด30บาท และได้เงินจาก Facebook400บาท บันทึกลงเงินสด และ กสิกรตามลำดับ");
    assert!(drafts[0].input.account.is_some());
    assert!(drafts[1].input.account.is_none());
    assert!(drafts[1].guidance.contains("เพิ่มบัญชี"));
}
#[test]
fn unsafe_or_ambiguous_phrases_never_become_a_partial_batch() {
    for source in [
        "",
        "ไม่ได้ซื้อไก่ทอด30บาท",
        "ถ้าซื้อข้าว30บาท",
        "ซื้อข้าว30บาท ไม่ใช่300บาท",
        "ซื้อไก่ทอด30หรือ40บาท",
        "ซื้อข้าว30บาทจากเงินสดหมวดอาหาร80บาท",
        "เมื่อวานซื้อข้าว30บาทวันนี้",
        "ซื้อไก่ทอด30ดอลลาร์",
        "ซื้อไก่ทอด30USD",
        "ซื้อไก่ทอด-30บาท",
        "ซื้อไก่ทอด- 30บาท",
        "ซื้อไก่ทอด0บาท",
        "ซื้อไก่ทอด30.001บาท",
        "ซื้อไก่ทอด1,20บาท",
        "ซื้อไก่ทอด1.2,00บาท",
        "ซื้อไก่ทอด30บาท และ",
        "ซื้อไก่ทอด30บาท และทำอย่างอื่น",
        "ซื้อข้าว30บาท จ่ายน้ำ20บาท",
        "ซื้อข้าว30บาทจากเงินสด และ โอน80",
        "ซื้อข้าว30บาทก่อนหักภาษี",
        "ได้เงินหลังหักณที่จ่าย400บาท",
        "ซื้อข้าว30บาท บันทึกลง \"เงินสด",
        "ซื้อข้าว90000000001บาท",
        "จ่าย 100 จาก เงินสด หมวด หนี้",
        "โอน 80 จาก เงินสด ไป เงินสด",
    ] {
        assert!(resolve(source).is_err(), "accepted {source}");
    }
}
#[test]
fn eight_entries_are_bounded_and_identical_purchases_are_not_deduplicated() {
    let text = ["ซื้อข้าว30บาท"; 8].join(" และ ");
    assert_eq!(batch(&text).len(), 8);
    assert!(resolve(&format!("{text} และซื้อข้าว30บาท")).is_err());
}
#[test]
fn transfers_keep_destination_and_never_gain_an_expense_category() {
    let drafts = batch("โอน 100 จาก กรุงไทย ไป เงินสด และซื้อข้าว30บาท จาก เงินสด หมวด อาหาร");
    assert_eq!(drafts[0].input.kind, TransactionKind::Transfer);
    assert_eq!(drafts[0].input.destination, Some(accounts()[0].id()));
    assert_eq!(drafts[0].input.category, None);
    assert!(missing_entry_fields(&drafts[0].input).is_empty());
}

#[test]
fn screenshot_and_conversational_connectors_preserve_each_account_and_date() {
    for connector in ["แล้วก็", "แล้ว", "และก็", "และ", "\n"] {
        let drafts = batch(&format!(
            "เมื่อวานซื้อกาแฟ 50 บาท บันทึกลง เงินสด {connector}วันนี้ได้เงินค่าจ้างวาดรูป 200 บาท บันทึกลงกรุงไทย"
        ));
        assert_eq!(drafts.len(), 2, "{connector}");
        assert_eq!(drafts[0].input.amount, "50.00");
        assert_eq!(drafts[0].input.date, "2025-12-31");
        assert_eq!(drafts[0].input.account, Some(accounts()[0].id()));
        assert_eq!(drafts[1].input.kind, TransactionKind::Income);
        assert_eq!(drafts[1].input.amount, "200.00");
        assert_eq!(drafts[1].input.date, "2026-01-01");
        assert_eq!(drafts[1].input.account, Some(accounts()[1].id()));
        assert_eq!(missing_entry_fields(&drafts[1].input), ["หมวดหมู่"]);
    }
    assert!(resolve("ซื้อกาแฟ50บาท บันทึกลงเงินสด วันนี้ได้เงิน200บาท บันทึกลงกรุงไทย").is_err());
}

#[test]
fn fully_specified_conversation_is_ready_to_preview_and_save() {
    let drafts = batch(
        "เมื่อวานซื้อกาแฟ 50 บาท บันทึกลง เงินสด หมวด อาหาร แล้วก็วันนี้ได้เงินค่าจ้างวาดรูป 200 บาท บันทึกลงกรุงไทย หมวด ฟรีแลนซ์",
    );
    assert_eq!(drafts.len(), 2);
    assert!(
        drafts
            .iter()
            .all(|d| missing_entry_fields(&d.input).is_empty())
    );
    assert_eq!(drafts[0].input.category, Some(Category::Food));
    assert_eq!(drafts[1].input.category, Some(Category::Freelance));
}
