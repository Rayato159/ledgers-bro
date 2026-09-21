#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;
use serde_json::{Value, json};

fn candidate() -> Value {
    json!({"intent":"expense","amount_text":"80","currency_text":null,"account_text":null,"destination_account_text":null,"category_text":null,"date_text":null,"description_text":null})
}
fn resolve(
    source: &str,
    candidates: Vec<Value>,
    accounts: &[Account],
) -> Result<QuickResolution, AppError> {
    let output = json!({"schema_version":"1","status":"proposal","candidates":candidates});
    resolve_model_proposal(
        source,
        &output.to_string(),
        "2026-09-21".parse().expect("date"),
        accounts,
    )
}
fn draft(result: QuickResolution) -> EntryInput {
    match result {
        QuickResolution::Choices { drafts, .. } => drafts.first().expect("choice").input.clone(),
        _ => panic!("model results must require explicit choice"),
    }
}
#[test]
fn natural_language_is_a_choice_with_no_implicit_account_or_category() {
    let input = draft(resolve("ซื้อกาแฟ 80 บาท", vec![candidate()], &[]).expect("grounded"));
    assert_eq!(input.amount, "80.00");
    assert_eq!(input.date, "2026-09-21");
    assert_eq!(input.account, None);
    assert_eq!(input.category, None);
    assert!(!input.is_complete());
}
#[test]
fn numeric_substrings_cannot_change_signed_decimal_or_grouped_amounts() {
    for source in [
        "กาแฟ 800",
        "กาแฟ 80.50",
        "กาแฟ -80",
        "กาแฟ 1,080",
        "กาแฟ 80,000",
    ] {
        let input = draft(resolve(source, vec![candidate()], &[]).expect("choice"));
        assert!(input.amount.is_empty(), "{source}");
    }
    let mut value = candidate();
    value["amount_text"] = json!("80.50");
    assert_eq!(
        draft(resolve("กาแฟ 80.50บาท", vec![value], &[]).expect("decimal")).amount,
        "80.50"
    );
}
#[test]
fn fuzzy_account_spelling_needs_a_user_choice_and_dates_are_host_resolved() {
    let accounts = [Account::new(
        "00000000-0000-0000-0000-000000000001".parse().expect("id"),
        AccountName::new("เงินสด").expect("name"),
        AccountKind::Cash,
    )];
    let mut value = candidate();
    value["account_text"] = json!("เงนสด");
    value["date_text"] = json!("เมื่อวาน");
    let input = draft(resolve("เมื่อวานจ่าย 80 จากเงนสด", vec![value], &accounts).expect("draft"));
    assert_eq!(input.account, None);
    assert_eq!(input.date, "2026-09-20");
}
#[test]
fn unknown_and_future_dates_never_silently_become_today() {
    for date in ["วันศุกร์", "2026-12-01"] {
        let mut value = candidate();
        value["date_text"] = json!(date);
        let input = draft(resolve(&format!("ซื้อกาแฟ 80 {date}"), vec![value], &[]).expect("draft"));
        assert!(input.date.is_empty());
    }
}
#[test]
fn tax_breakdowns_and_non_baht_are_rejected_even_if_model_proposes_them() {
    for source in ["ค่าบริการ 80 VAT 7 หัก ณ ที่จ่าย 3", "เงินเดือนก่อนหัก 80", "ภาษี 80"]
    {
        assert!(resolve(source, vec![candidate()], &[]).is_err());
    }
    let mut value = candidate();
    value["currency_text"] = json!("USD");
    assert!(resolve("ค่าแอป 80 USD", vec![value], &[]).is_err());
}
#[test]
fn alternatives_are_never_mapped_to_multiple_writes() {
    let first = candidate();
    let mut second = candidate();
    second["amount_text"] = json!("90");
    match resolve("กาแฟ 80 หรือ 90", vec![first, second], &[]).expect("alternatives")
    {
        QuickResolution::Choices { drafts, .. } => assert_eq!(drafts.len(), 2),
        _ => panic!("must ask the user"),
    }
}

#[test]
fn negations_and_scope_guards_do_not_depend_on_the_model_obeying_its_prompt() {
    for source in [
        "ไม่ได้ซื้อกาแฟ 80 บาท",
        "ถ้าซื้อกาแฟ 80 บาท",
        "ลบรายการ 80",
        "กาแฟ 80 ข้าว 60 สองรายการ",
    ] {
        assert!(resolve(source, vec![candidate()], &[]).is_err());
    }
}

#[test]
fn invented_labels_are_left_blank_but_never_treated_as_grounded_account_ids() {
    let accounts = [Account::new(
        "00000000-0000-0000-0000-000000000001".parse().expect("id"),
        AccountName::new("เงินสด").expect("name"),
        AccountKind::Cash,
    )];
    let mut value = candidate();
    value["account_text"] = json!("เงินสด");
    value["category_text"] = json!("อาหาร");
    let input = draft(resolve("กาแฟ 80 จากเงนสด", vec![value], &accounts).expect("partial choice"));
    assert_eq!(input.amount, "80.00");
    assert_eq!(input.account, None);
    assert_eq!(input.category, None);
    let mut value = candidate();
    value["amount_text"] = json!("90");
    assert!(resolve("กาแฟ 80", vec![value], &[]).is_err());
}

#[test]
fn omitted_currency_or_unparsed_date_cannot_silently_become_baht_or_today() {
    assert!(resolve("กาแฟ 80 USD", vec![candidate()], &[]).is_err());
    let input = draft(resolve("วันศุกร์ซื้อกาแฟ 80", vec![candidate()], &[]).expect("partial draft"));
    assert!(input.date.is_empty());
}
