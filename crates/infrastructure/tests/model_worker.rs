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
    assert!(
        matches!(block_on(model.resolve(&ledger, "เมื่อวานซื้อกาแฟ 50 บาท บันทึกลง เงินสด แล้วก็วันนี้ได้เงินค่าจ้างวาดรูป 200 บาท บันทึกลงกรุงไทย".into(), ModelOperation::default())), Ok(Response::Resolved(QuickResolution::Batch { drafts, .. })) if drafts.len() == 2)
    );
    let failure = block_on(model.resolve(
        &ledger,
        "ซื้อข้าว30บาท แล้วก็คำสั่งไม่สมบูรณ์".into(),
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
        credit_cycle: None,
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

#[test]
fn action_prompts_work_without_model_and_failures_never_fall_back_to_one_purchase() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let model = ModelWorker::start(directory.path().join("models")).expect("worker");
    let ledger = LedgerWorker::start(directory.path().join("ledger.sqlite3")).expect("ledger");
    for text in [
        "เพิ่มบัญชี ชื่อ เงินสด ประเภท เงินสด ยอดเริ่มต้น 1000",
        "เพิ่มลูกหนี้ ชื่อ เอ ยอด 500",
        "ยกเลิกรายการ รายการ กาแฟ",
        "เพิ่มลูกหนี้ ชื่อ เอ เรื่อง ค่าเรียน ยอด 500\nเอคืนหนี้ 50 บาท เข้า เงินสด",
    ] {
        assert!(matches!(
            block_on(model.resolve(&ledger, text.into(), ModelOperation::default())),
            Ok(Response::Resolved(QuickResolution::Actions { .. }))
        ));
    }
    let error = block_on(model.resolve(
        &ledger,
        "สมชายคืนหนี้ 100 บาท แล้วก็ข้อความไม่ครบ".into(),
        ModelOperation::default(),
    ))
    .expect_err("whole message");
    assert!(!error.to_string().contains("ดาวน์โหลด"));
    let Response::Dashboard(view) = block_on(ledger.request(Command::Load)).expect("view") else {
        unreachable!()
    };
    assert!(view.entries.is_empty());
    assert!(view.receivables.is_empty());
    assert!(view.accounts.is_empty());
}

#[test]
fn long_receipt_blocks_bypass_ai_and_keep_every_item() {
    use ledger_application::{analyze_receipt, format_receipt_prompt};
    let directory = tempfile::tempdir().expect("directory");
    let model = ModelWorker::start(directory.path().join("models")).expect("worker");
    let ledger = LedgerWorker::start(directory.path().join("ledger.sqlite3")).expect("ledger");
    let today = "2026-09-20".parse().expect("date");
    let lines = (1..=40)
        .map(|n| format!("Coffee number {n} 1.00"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\nGrand Total 40.00";
    let text = format_receipt_prompt(&analyze_receipt(&lines, today).expect("analysis"), today, 1);
    assert!(text.chars().count() > 1000);
    let response = block_on(model.resolve(&ledger, text.clone(), ModelOperation::default()))
        .expect("receipt needs no AI weights");
    let Response::Resolved(QuickResolution::Batch { drafts, .. }) = response else {
        unreachable!()
    };
    assert_eq!(
        drafts[0]
            .input
            .receipt
            .as_ref()
            .expect("receipt")
            .lines
            .len(),
        40
    );
    let error = block_on(model.resolve(
        &ledger,
        text.replace("[จบใบเสร็จ]", ""),
        ModelOperation::default(),
    ))
    .expect_err("malformed receipt");
    assert!(!error.to_string().contains("ดาวน์โหลด"));
}
