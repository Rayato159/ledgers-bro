#![allow(clippy::expect_used)]
use futures_executor::block_on;
use ledger_application::*;
use ledger_infrastructure::{LOCAL_MODEL_FILENAME, LedgerWorker, ModelWorker};
use std::sync::atomic::Ordering;

#[test]
fn missing_model_keeps_commands_and_bounded_thai_batches_available() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let model = ModelWorker::start(directory.path().join("models")).expect("worker");
    let ledger = LedgerWorker::start(directory.path().join("ledger.sqlite3")).expect("ledger");
    assert_eq!(
        block_on(model.availability()).expect("status"),
        ModelAvailability::Missing
    );
    assert!(matches!(
        block_on(model.resolve(&ledger, "กาแฟ80บาท".into(), ModelOperation::default())),
        Ok(Response::Resolved(QuickResolution::Draft { .. }))
    ));
    assert!(
        block_on(model.resolve(
            &ledger,
            "เช้านี้แวะซื้อกาแฟจ่ายไป 80 บาท".into(),
            ModelOperation::default()
        ))
        .is_err()
    );
    assert!(
        matches!(block_on(model.resolve(&ledger, "ซื้อไก่ทอดไป 30 บาท และได้เงินจาก Facebook 400 บาท บันทึกลงเงินสด และ กรุงไทยตามลำดับ".into(), ModelOperation::default())), Ok(Response::Resolved(QuickResolution::Batch { drafts, .. })) if drafts.len() == 2)
    );
    let failure = block_on(model.resolve(
        &ledger,
        "ซื้อข้าว30บาท และคำสั่งไม่สมบูรณ์".into(),
        ModelOperation::default(),
    ))
    .expect_err("whole batch must fail");
    assert!(!failure.to_string().contains("ดาวน์โหลด"));
}

#[test]
fn cancellation_and_corrupt_weights_never_initialize_native_inference() {
    let directory = tempfile::tempdir().expect("temporary directory");
    std::fs::write(directory.path().join(LOCAL_MODEL_FILENAME), b"not a model").expect("fixture");
    let model = ModelWorker::start(directory.path().to_owned()).expect("worker");
    assert_eq!(
        block_on(model.availability()).expect("status"),
        ModelAvailability::Missing
    );
    assert!(block_on(model.propose("ซื้อกาแฟ 80 บาท".into(), ModelOperation::default())).is_err());
    let operation = ModelOperation::default();
    operation.cancelled.store(true, Ordering::Relaxed);
    assert!(block_on(model.install(operation)).is_err());
    assert_eq!(
        std::fs::read(directory.path().join(LOCAL_MODEL_FILENAME)).expect("unchanged file"),
        b"not a model"
    );
}

#[test]
fn model_interpretation_and_preview_do_not_write_until_an_explicit_commit() {
    use ledger_domain::AccountKind;
    let directory = tempfile::tempdir().expect("temporary directory");
    let worker = LedgerWorker::start(directory.path().join("ledger.sqlite3")).expect("worker");
    block_on(worker.request(Command::CreateAccount {
        name: "เงินสด".into(),
        kind: AccountKind::Cash,
        opening: "1000".into(),
    }))
    .expect("account");
    let before = block_on(worker.request(Command::Load)).expect("snapshot");
    let output = serde_json::json!({"schema_version":"1","status":"proposal","candidates":[{"intent":"expense","amount_text":"80","currency_text":"บาท","account_text":"เงินสด","destination_account_text":null,"category_text":"อาหาร","date_text":null,"description_text":null}]}).to_string();
    let response = block_on(worker.request(Command::ResolveModel {
        source: "จายค่าอาหาร 80 บาทจากเงินสด".into(),
        output,
    }))
    .expect("proposal");
    let Response::Resolved(QuickResolution::Choices { drafts, .. }) = response else {
        unreachable!()
    };
    let prepared =
        block_on(worker.request(Command::Preview(drafts[0].input.clone()))).expect("preview");
    let after = block_on(worker.request(Command::Load)).expect("snapshot");
    if let (Response::Dashboard(before), Response::Dashboard(after)) = (before, after) {
        assert_eq!(before, after);
    } else {
        unreachable!()
    }
    let Response::Prepared(prepared) = prepared else {
        unreachable!()
    };
    block_on(worker.request(Command::Commit(prepared.clone()))).expect("commit");
    block_on(worker.request(Command::Commit(prepared))).expect("idempotent retry");
    let Response::Dashboard(after) = block_on(worker.request(Command::Load)).expect("snapshot")
    else {
        unreachable!()
    };
    assert_eq!(after.expenses.to_string(), "80.00");
    assert_eq!(after.accounts[0].balance.to_string(), "920.00");
}
