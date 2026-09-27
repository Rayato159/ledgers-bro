#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use serde_json::{Value, json};

fn item(amount: &str, description: Value) -> Value {
    json!({"intent":"expense","amount_text":amount,"currency_text":null,"account_text":null,"destination_account_text":null,"category_text":null,"date_text":null,"description_text":description})
}
fn resolve(source: &str, entries: Vec<Value>) -> Result<QuickResolution, AppError> {
    resolve_model_proposal(
        source,
        &json!({"schema_version":"2","status":"proposal","entries":entries}).to_string(),
        "2026-09-28".parse().expect("date"),
        &[],
    )
}
#[test]
fn batch_keeps_each_purchase_and_repeated_entries_in_source_order() {
    let entries = vec![
        item("80", json!("กาแฟ")),
        item("60", json!("ข้าว")),
        item("80", json!("กาแฟ")),
    ];
    let QuickResolution::Batch { drafts, .. } =
        resolve("ซื้อกาแฟ 80 ข้าว 60 แล้วกาแฟอีก 80", entries).expect("batch")
    else {
        panic!("must be a batch")
    };
    assert_eq!(drafts.len(), 3);
    assert_eq!(
        drafts
            .iter()
            .map(|d| d.input.note.as_str())
            .collect::<Vec<_>>(),
        ["กาแฟ", "ข้าว", "กาแฟ"]
    );
    assert_eq!(
        drafts
            .iter()
            .map(|d| d.input.amount.as_str())
            .collect::<Vec<_>>(),
        ["80.00", "60.00", "80.00"]
    );
    assert!(
        drafts
            .iter()
            .all(|d| d.input.account.is_none() && d.input.category.is_none())
    );
}
#[test]
fn missing_or_invented_description_keeps_original_context() {
    for description in [Value::Null, json!("Invented description")] {
        let source = "ซื้อสมุด 80";
        let QuickResolution::Batch { drafts, .. } =
            resolve(source, vec![item("80", description)]).expect("review")
        else {
            panic!("batch")
        };
        assert_eq!(drafts[0].input.note, source);
    }
}
#[test]
fn ambiguous_amounts_and_oversized_or_malformed_batches_never_become_partial_drafts() {
    assert!(
        resolve(
            "กาแฟ 80 หรือ 60",
            vec![item("80", json!("กาแฟ")), item("60", json!("กาแฟ"))]
        )
        .is_err()
    );
    assert!(resolve("กาแฟ 80", vec![item("80", Value::Null); 9]).is_err());
    assert!(resolve("กาแฟ 80", vec![]).is_err());
    let mut invalid = item("60", json!("ข้าว"));
    invalid["save"] = json!(true);
    assert!(resolve("กาแฟ 80 ข้าว 60", vec![item("80", json!("กาแฟ")), invalid]).is_err());
    let output = r#"{"schema_version":"2","status":"proposal","entries":["#;
    assert!(
        resolve_model_proposal("กาแฟ 80", output, "2026-09-28".parse().expect("date"), &[])
            .is_err()
    );
}
#[test]
fn legacy_candidates_remain_alternatives_not_additional_purchases() {
    let output = json!({"schema_version":"1","status":"proposal","candidates":[item("80", Value::Null),item("60", Value::Null)]});
    assert!(
        matches!(resolve_model_proposal("กาแฟ 80 หรือ 60", &output.to_string(), "2026-09-28".parse().expect("date"), &[]).expect("legacy"), QuickResolution::Choices { drafts, .. } if drafts.len() == 2)
    );
}
