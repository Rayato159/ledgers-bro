#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;

fn accounts() -> Vec<Account> {
    ["เงินสด", "ธนาคาร ออมเงิน"]
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
    resolve_quick_entry(text, "2026-01-01".parse().expect("date"), &accounts())
}
fn draft(result: QuickResolution) -> EntryInput {
    match result {
        QuickResolution::Draft { input, .. } => input,
        _ => panic!("expected draft"),
    }
}

#[test]
fn strict_command_supports_quoted_names_and_frozen_relative_date() {
    let input = draft(
        resolve("จ่าย 120.50 จาก \"ธนาคาร ออมเงิน\" หมวด อาหาร วันที่ เมื่อวาน โน้ต \"ข้าวกลางวัน\"")
            .expect("command"),
    );
    assert_eq!(input.amount, "120.50");
    assert_eq!(input.account, Some(accounts()[1].id()));
    assert_eq!(input.category, Some(Category::Food));
    assert_eq!(input.date, "2025-12-31");
    assert_eq!(input.note, "ข้าวกลางวัน");
}

#[test]
fn malformed_input_never_saves_a_recognized_prefix() {
    for text in [
        "จ่าย 80 จาก เงินสด หมวด อาหาร และ 30",
        "จ่าย 12O จาก เงินสด หมวด อาหาร",
        "จ่าย 80 จาก เงินสด หมวด อาหาร โน้ต \"ไม่ปิด",
        "จ่าย 80 จาก เงินสด หมวด อาหาร โน้ต \"a\"x",
        "จ่าย 80 จาก เงินสด หมวด อาหาร วันนี้",
        "กาแฟ 80 หรือ 90",
        "ลบ รายการทั้งหมด",
        "เงินเดือน gross 40000 net 38000",
        "คำนวณภาษี 30000",
        "รับ 1040 เข้า เงินสด หมวด ฟรีแลนซ์ VAT 70 หัก ณ ที่จ่าย 30",
        "จ่าย 1040 จาก เงินสด หมวด อื่นๆ ฐาน 1000 VAT 70 WHT 30",
        "ค่าบริการ 1000 + VAT 7% - หัก ณ ที่จ่าย 3%",
        "โอน 80",
        "กาแฟ 1,2",
        "กาแฟ 80.001",
    ] {
        assert!(resolve(text).is_err(), "accepted {text}");
    }
}

#[test]
fn unknown_names_and_typos_require_explicit_choices() {
    let input = draft(resolve("จ่าย 80 จาก กรุง หมวด อาหาน").expect("partial draft"));
    assert_eq!(input.account, None);
    assert_eq!(input.category, None);
    let shorthand = draft(resolve("กาแฟ 80").expect("shorthand"));
    assert_eq!(shorthand.account, None);
    assert_eq!(shorthand.category, None);
    assert_eq!(shorthand.note, "กาแฟ");
    assert_eq!(shorthand.date, "2026-01-01");
}

#[test]
fn transfers_and_debt_are_not_guessed_as_expenses() {
    let input = draft(resolve("โอน 1000 จาก \"ธนาคาร ออมเงิน\" ไป เงินสด").expect("transfer"));
    assert_eq!(input.kind, TransactionKind::Transfer);
    assert_eq!(input.category, None);
    assert_eq!(input.destination, Some(accounts()[0].id()));
    assert!(resolve("โอน 80 จาก เงินสด ไป เงินสด").is_err());
    assert_eq!(
        resolve("จ่าย 100 จาก เงินสด หมวด หนี้"),
        Err(AppError::Rule(DomainError::DebtNeedsTransfer))
    );
}

#[test]
fn commands_are_deterministic_for_the_same_context() {
    let source = "จ่าย 80 จาก เงินสด หมวด อาหาน วันที่ เมื่อวาน";
    let baseline = resolve(source);
    for _ in 0..100 {
        assert_eq!(resolve(source), baseline);
    }
    assert_eq!(resolve("สรุป เดือนนี้"), Ok(QuickResolution::Summary));
    assert_eq!(resolve("ช่วยเหลือ"), Ok(QuickResolution::Help));
}

#[test]
fn spoken_shorthand_allows_missing_spaces_and_an_explicit_baht_unit() {
    for phrase in [
        "กาแฟ80.50บาท",
        "กาแฟ 80.50 บาท",
        "กาแฟ80.50",
        " กาแฟ 80.50บาท ",
    ] {
        let input = draft(resolve(phrase).expect("numeric spoken shorthand"));
        assert_eq!(input.amount, "80.50");
        assert_eq!(input.note, "กาแฟ");
        assert!(input.account.is_none());
        assert!(input.category.is_none());
        assert!(!input.is_complete());
    }
}

#[test]
fn spoken_shorthand_never_guesses_numbers_currency_or_a_second_instruction() {
    for phrase in [
        "กาแฟแปดสิบบาท",
        "กาแฟ80บาทหรือ90บาท",
        "กาแฟ80บาทไม่ใช่800บาท",
        "กาแฟ80บาทโอน100บาท",
        "กาแฟ80ดอลลาร์",
        "กาแฟ80.005บาท",
        "กาแฟ8 0บาท",
        "กาแฟ0บาท",
        "กาแฟ-80บาท",
    ] {
        assert!(
            resolve(phrase).is_err(),
            "accepted ambiguous speech: {phrase}"
        );
    }
}
