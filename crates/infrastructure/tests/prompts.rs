#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{RandomIds, SqliteLedger};
use rusqlite::Connection;
use tempfile::TempDir;
struct FixedClock;
impl Clock for FixedClock {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok("2026-09-24".parse()?)
    }
}
type App = LedgerApplication<SqliteLedger, FixedClock, RandomIds>;
fn app(repo: SqliteLedger) -> App {
    LedgerApplication::new(repo, FixedClock, RandomIds)
}
fn view(app: &mut App) -> Dashboard {
    match app.execute(Command::Load).expect("view") {
        Response::Dashboard(v) => v,
        _ => panic!("view"),
    }
}
fn drafts(app: &mut App, text: &str) -> Vec<PromptDraft> {
    match app.execute(Command::Resolve(text.into())).expect(text) {
        Response::Resolved(QuickResolution::Actions { drafts, .. }) => drafts,
        other => panic!("{other:?}"),
    }
}
fn preview(app: &mut App, text: &str) -> PromptPlan {
    let before = view(app);
    let d = drafts(app, text);
    let p = match app.execute(Command::PreviewPrompt(d)).expect(text) {
        Response::PreparedPrompt(p) => p,
        _ => panic!("plan"),
    };
    assert_eq!(view(app), before, "preview must not write");
    p
}
fn run(app: &mut App, text: &str) -> PromptPlan {
    let p = preview(app, text);
    app.execute(Command::CommitPrompt(p.clone())).expect(text);
    p
}
fn setup(repo: SqliteLedger) -> App {
    let mut app = app(repo);
    run(
        &mut app,
        "เพิ่มบัญชี ชื่อ เงินสด ประเภท เงินสด ยอดเริ่มต้น 20000\nเพิ่มบัญชี ชื่อ กรุงไทย ประเภท ธนาคาร ยอดเริ่มต้น 1000",
    );
    app
}
#[test]
fn credit_card_prompt_requires_dates_and_commits_them_atomically() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let missing = drafts(&mut app, "เพิ่มบัญชี ชื่อ Visa ประเภท บัตรเครดิต ยอดหนี้ 0");
    assert!(app.execute(Command::PreviewPrompt(missing)).is_err());
    assert!(view(&mut app).accounts.is_empty());
    run(
        &mut app,
        "เพิ่มบัญชี ชื่อ Visa ประเภท บัตรเครดิต ยอดหนี้ 0 วันตัดรอบ 20 วันชำระ 5",
    );
    let v = view(&mut app);
    assert_eq!(
        v.accounts[0].account.credit_cycle(),
        Some(CreditCardCycle::new(20, 5).expect("cycle"))
    );
}

#[test]
fn mixed_create_account_expense_income_transfer_is_one_idempotent_commit() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let p = run(
        &mut app,
        "เพิ่มบัญชี ชื่อ เงินสด ประเภท เงินสด ยอดเริ่มต้น 20000 แล้วก็เพิ่มบัญชี ชื่อ กรุงไทย ประเภท ธนาคาร ยอดเริ่มต้น 1000 แล้วจ่าย 80 จาก เงินสด หมวด อาหาร แล้วรับ 400 เข้า กรุงไทย หมวด ฟรีแลนซ์ และโอน 100 จาก เงินสด ไป กรุงไทย",
    );
    let saved = view(&mut app);
    assert_eq!(saved.income.to_string(), "400.00");
    assert_eq!(saved.expenses.to_string(), "80.00");
    assert_eq!(saved.net_worth.to_string(), "21320.00");
    app.execute(Command::CommitPrompt(p)).expect("retry");
    assert_eq!(view(&mut app), saved);
}
#[test]
fn recurring_create_pay_change_count_link_and_stop_use_same_rules() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    run(
        &mut app,
        "เพิ่มรายจ่ายประจำ ชื่อ ค่ารถ ยอด 4500 บาท ทุกวันที่ 5 จำนวนงวด 12 เริ่ม 2026-10 จาก เงินสด หมวด อื่นๆ\nจ่ายงวด ชื่อ ค่ารถ งวด 2026-10\nตั้งจำนวนงวด ชื่อ ค่ารถ จำนวนงวด 2",
    );
    let v = view(&mut app);
    let progress = recurring_progress(&v);
    assert_eq!(progress[0].paid, 1);
    assert_eq!(progress[0].remaining, Some(1));
    run(
        &mut app,
        "จ่าย 4500 จาก เงินสด หมวด อื่นๆ โน้ต ค่ารถพฤศจิกายน\nผูกรายจ่ายเดิม ชื่อ ค่ารถ งวด 2026-11 รายการ ค่ารถพฤศจิกายน",
    );
    assert!(recurring_progress(&view(&mut app))[0].completed);
    run(
        &mut app,
        "ตั้งจำนวนงวด ชื่อ ค่ารถ จำนวนงวด ไม่กำหนด\nหยุดแผน ชื่อ ค่ารถ ตั้งแต่ 2026-12",
    );
    assert_eq!(
        view(&mut app).recurring[0]
            .stopped_from()
            .expect("stopped")
            .to_string(),
        "2026-12"
    );
}
#[test]
fn receivables_existing_new_lending_and_repayment_with_interest() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    run(
        &mut app,
        "เพิ่มลูกหนี้ ชื่อ สมชาย เรื่อง ยืมซื้อคอม ยอด 9000 บาท จำนวนงวด 9 เก็บทุกวันที่ 5 เริ่ม 2026-10\nให้สมหญิงยืม 3000 บาท จาก เงินสด เรื่อง ยืมซื้อของ\nสมชายคืนหนี้ 1000 บาท เข้า กรุงไทย ดอกเบี้ย 50 บาท\nสมหญิงชำระหนี้ 500 บาท เข้า เงินสด",
    );
    let v = view(&mut app);
    let summary = receivable_summary(&v).expect("summary");
    assert_eq!(summary.outstanding.to_string(), "10500.00");
    assert_eq!(v.income.to_string(), "50.00");
    assert_eq!(v.expenses.to_string(), "0.00");
    assert_eq!(v.net_worth.to_string(), "30050.00");
}
#[test]
fn reversal_and_delete_show_concrete_impact_and_require_commit() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    run(
        &mut app,
        "เพิ่มบัญชี ชื่อ สำรอง ประเภท เงินสด ยอดเริ่มต้น 0\nจ่าย 80 จาก เงินสด หมวด อาหาร โน้ต กาแฟ\nโอน 100 จาก เงินสด ไป สำรอง",
    );
    run(
        &mut app,
        "ยกเลิกรายการ รายการ กาแฟ วันที่ 2026-09-24 ยอด 80 บาท",
    );
    assert_eq!(view(&mut app).expenses.to_string(), "0.00");
    let p = preview(&mut app, "ลบบัญชี ชื่อ สำรอง");
    assert!(p.descriptions[0].contains("ลบถาวร"));
    assert!(p.descriptions[0].contains("เงินสด"));
    app.execute(Command::CommitPrompt(p)).expect("delete");
    assert_eq!(view(&mut app).accounts.len(), 2);
    assert_eq!(view(&mut app).net_worth.to_string(), "21000.00");
}
#[test]
fn missing_fields_are_editable_and_ambiguous_names_never_choose_first() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    let mut d = drafts(&mut app, "เพิ่มลูกหนี้ ชื่อ สมชาย ยอด 100");
    assert!(d[0].questions().iter().any(|q| q.contains("หนี้อะไร")));
    assert!(app.execute(Command::PreviewPrompt(d.clone())).is_err());
    d[0].set("description", "ค่าข้าว".into());
    let p = match app.execute(Command::PreviewPrompt(d)).expect("fixed") {
        Response::PreparedPrompt(p) => p,
        _ => panic!("review"),
    };
    app.execute(Command::CommitPrompt(p)).expect("commit");
    run(&mut app, "เพิ่มลูกหนี้ ชื่อ สมชาย เรื่อง ค่าหนัง ยอด 200");
    let d = drafts(&mut app, "สมชายคืนหนี้ 50 บาท เข้า เงินสด");
    let error = app
        .execute(Command::PreviewPrompt(d))
        .expect_err("ambiguous")
        .to_string();
    assert!(error.contains("2 รายการ"), "{error}");
    let before = view(&mut app);
    assert_eq!(
        receivable_summary(&before)
            .expect("summary")
            .outstanding
            .to_string(),
        "300.00"
    );
    let d = drafts(&mut app, "ตั้งจำนวนงวด ชื่อ ค่ารถ");
    assert!(d[0].questions().iter().any(|q| q.contains("จำนวนงวด")));
}
#[test]
fn late_invalid_action_and_sql_failure_leave_no_partial_writes_and_retry_is_safe() {
    let temp = TempDir::new().expect("temp");
    let path = temp.path().join("ledger.db");
    let mut app = setup(SqliteLedger::open(&path).expect("db"));
    let before = view(&mut app);
    let d = drafts(
        &mut app,
        "เพิ่มบัญชี ชื่อ ใหม่ ประเภท เงินสด ยอดเริ่มต้น 100\nให้ยืมเงิน ชื่อ เอ เรื่อง ค่าเรียน ยอด 500 จาก ไม่มีบัญชี",
    );
    assert!(app.execute(Command::PreviewPrompt(d)).is_err());
    assert_eq!(view(&mut app), before);
    let p = preview(
        &mut app,
        "เพิ่มบัญชี ชื่อ ใหม่ ประเภท เงินสด ยอดเริ่มต้น 100\nเพิ่มลูกหนี้ ชื่อ เอ เรื่อง ค่าเรียน ยอด 500",
    );
    let raw = Connection::open(&path).expect("connection");
    raw.execute_batch("CREATE TRIGGER fail_prompt BEFORE INSERT ON postings WHEN NEW.system_book='receivable' BEGIN SELECT RAISE(ABORT, 'test'); END;").expect("trigger");
    assert!(app.execute(Command::CommitPrompt(p.clone())).is_err());
    assert_eq!(view(&mut app), before);
    raw.execute_batch("DROP TRIGGER fail_prompt").expect("drop");
    app.execute(Command::CommitPrompt(p.clone()))
        .expect("retry");
    let saved = view(&mut app);
    app.execute(Command::CommitPrompt(p)).expect("idempotent");
    assert_eq!(view(&mut app), saved);
}
#[test]
fn stale_review_requires_new_preview_including_metadata_only_changes() {
    let temp = TempDir::new().expect("temp");
    let path = temp.path().join("ledger.db");
    let mut first = setup(SqliteLedger::open(&path).expect("db"));
    let mut second = app(SqliteLedger::open(&path).expect("second"));
    let stale = preview(&mut first, "เพิ่มบัญชี ชื่อ ใหม่ ประเภท เงินสด ยอดเริ่มต้น 0");
    run(
        &mut second,
        "เพิ่มรายจ่ายประจำ ชื่อ ค่าเน็ต ยอด 500 ทุกวันที่ 30 เริ่ม 2026-09 จาก เงินสด หมวด อื่นๆ",
    );
    assert!(
        first
            .execute(Command::CommitPrompt(stale))
            .expect_err("stale")
            .to_string()
            .contains("เปลี่ยน")
    );
    assert_eq!(view(&mut first).accounts.len(), 2);
}
#[test]
fn invalid_or_unconsumed_text_cannot_become_a_partial_command() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    let before = view(&mut app);
    for text in [
        "อย่าเพิ่มบัญชี ชื่อ ใหม่ ประเภท เงินสด ยอดเริ่มต้น 0",
        "สมมติให้สมชายยืม 500 บาท",
        "เพิ่มลูกหนี้ ชื่อ เอ ยอด 100 และไม่ต้องบันทึก",
        "เพิ่มบัญชี ชื่อ ใหม่ ประเภท เงินสด ยอดเริ่มต้น 100 ซื้อข้าว 20",
        "เพิ่มลูกหนี้ ชื่อ เอ ยอด 1,00 บาท",
        "เพิ่มลูกหนี้ ชื่อ เอ ยอด 100 ยอด 200",
        "เพิ่มลูกหนี้ ชื่อ \"เอ ยอด 100",
        "เพิ่มบัญชี ชื่อ ใหม่ ประเภท เงินสด ยอดเริ่มต้น 0\nทำอะไรบางอย่าง",
        "เพิ่มลูกหนี้ ชื่อ เอ ยอด 100 USD",
    ] {
        assert!(
            app.execute(Command::Resolve(text.into())).is_err(),
            "{text}"
        );
    }
    assert_eq!(view(&mut app), before);
}
#[test]
fn quoted_names_and_descriptions_and_relative_dates_are_preserved() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    run(
        &mut app,
        "เมื่อวานเพิ่มลูกหนี้ ชื่อ \"เอ และ บี\" เรื่อง \"ค่าอาหารและขนม\" ยอด 1,000.50 บาท",
    );
    let v = view(&mut app);
    assert_eq!(v.receivables[0].debtor().as_str(), "เอ และ บี");
    assert_eq!(v.receivables[0].description().as_str(), "ค่าอาหารและขนม");
    assert_eq!(v.receivables[0].opened().to_string(), "2026-09-23");
}
#[test]
fn every_advertised_example_resolves_to_its_action_without_a_model() {
    let mut app = setup(SqliteLedger::in_memory().expect("db"));
    for kind in PromptKind::ALL {
        if matches!(
            kind,
            PromptKind::Expense | PromptKind::Income | PromptKind::Transfer
        ) {
            assert!(app.execute(Command::Resolve(kind.example().into())).is_ok());
        } else {
            let d = drafts(&mut app, kind.example());
            assert_eq!(d.len(), 1);
            assert_eq!(d[0].kind, kind);
        }
    }
}
#[test]
fn prompt_schema_upgrade_preserves_existing_receivable_and_settlement_history() {
    let temp = TempDir::new().expect("temp");
    let path = temp.path().join("ledger.db");
    let mut original = setup(SqliteLedger::open(&path).expect("db"));
    run(
        &mut original,
        "เพิ่มลูกหนี้ ชื่อ เอ เรื่อง ค่าเรียน ยอด 500\nเอคืนหนี้ 50 บาท เข้า เงินสด",
    );
    let expected = view(&mut original);
    drop(original);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("DROP TABLE user_preferences; DROP TRIGGER lock_currency_accounts; DROP TRIGGER lock_currency_recurring; DROP TRIGGER lock_currency_receivables; DROP TABLE ledger_settings; DROP TABLE prompt_submissions; PRAGMA user_version=4;")
        .expect("v4");
    raw.execute_batch("ALTER TABLE accounts DROP COLUMN payment_day; ALTER TABLE accounts DROP COLUMN closing_day;").expect("remove v8 columns for old schema fixture");
    let mut migrated = app(SqliteLedger::open(&path).expect("migrate"));
    assert_eq!(view(&mut migrated), expected);
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .expect("version"),
        8
    );
}
