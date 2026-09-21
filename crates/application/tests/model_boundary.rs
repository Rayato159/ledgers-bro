#![allow(clippy::expect_used)]
use ledger_application::model_contract::*;
use serde_json::json;

fn proposal() -> serde_json::Value {
    json!({"schema_version":"1","status":"proposal","candidates":[{"intent":"expense","amount_text":"80","currency_text":null,"account_text":null,"destination_account_text":null,"category_text":null,"date_text":null,"description_text":"กาแฟ"}]})
}
#[test]
fn grounded_proposal_is_only_a_validated_interpretation() {
    assert!(validate_proposal("กาแฟ 80", &proposal().to_string()).is_ok());
}
#[test]
fn invented_text_and_extra_tool_fields_are_rejected() {
    let mut value = proposal();
    value["candidates"][0]["account_text"] = json!("เงินสด");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
    let mut value = proposal();
    value["sql"] = json!("DELETE FROM accounts");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
    let mut value = proposal();
    value["candidates"][0]["tool"] = json!("save");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
}
#[test]
fn required_nullable_keys_must_still_be_present() {
    let mut value = proposal();
    value["candidates"][0]
        .as_object_mut()
        .expect("object")
        .remove("currency_text");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
}
#[test]
fn schema_version_status_cardinality_and_field_limits_are_enforced() {
    for value in [
        json!({"schema_version":"2","status":"unsupported","candidates":[]}),
        json!({"schema_version":"1","status":"proposal","candidates":[]}),
    ] {
        assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
    }
    let mut value = proposal();
    value["status"] = json!("unsupported");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
    let mut value = proposal();
    value["candidates"] = json!(vec![value["candidates"][0].clone(); 4]);
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
    assert!(validate_proposal("กาแฟ 80", &" ".repeat(16_385)).is_err());
}
#[test]
fn summary_cannot_smuggle_write_fields() {
    let mut value = proposal();
    value["candidates"][0]["intent"] = json!("summary");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
    let mut value = proposal();
    value["candidates"][0]["destination_account_text"] = json!("กาแฟ");
    assert!(validate_proposal("กาแฟ 80", &value.to_string()).is_err());
}
