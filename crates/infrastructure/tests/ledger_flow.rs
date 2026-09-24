#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::*;
use rusqlite::Connection;
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

struct FixedClock;
impl Clock for FixedClock {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok("2026-09-20".parse()?)
    }
}
type Application = LedgerApplication<SqliteLedger, FixedClock, RandomIds>;
fn app(repo: SqliteLedger) -> Application {
    LedgerApplication::new(repo, FixedClock, RandomIds)
}
fn snapshot(app: &mut Application) -> Dashboard {
    match app.execute(Command::Load).expect("load") {
        Response::Dashboard(view) => view,
        _ => panic!("dashboard response"),
    }
}
fn account(app: &mut Application, name: &str, kind: AccountKind, opening: &str) -> AccountId {
    app.execute(Command::CreateAccount {
        name: name.into(),
        kind,
        opening: opening.into(),
    })
    .expect("create account");
    snapshot(app)
        .accounts
        .iter()
        .find(|a| a.account.name().as_str() == name)
        .expect("created account")
        .account
        .id()
}
fn preview(
    app: &mut Application,
    kind: TransactionKind,
    account: AccountId,
    amount: &str,
    category: Option<Category>,
    destination: Option<AccountId>,
) -> PreparedEntry {
    let input = EntryInput {
        kind,
        amount: amount.into(),
        account: Some(account),
        destination,
        category,
        date: "2026-09-20".into(),
        note: "test".into(),
        receipt: None,
    };
    match app.execute(Command::Preview(input)).expect("preview") {
        Response::Prepared(prepared) => prepared,
        _ => panic!("prepared response"),
    }
}
fn record(
    app: &mut Application,
    kind: TransactionKind,
    account: AccountId,
    amount: &str,
    category: Option<Category>,
    destination: Option<AccountId>,
) -> EntryId {
    let prepared = preview(app, kind, account, amount, category, destination);
    let id = prepared.entry.id();
    app.execute(Command::Commit(prepared)).expect("commit");
    id
}
fn minor(view: &Dashboard, id: AccountId) -> i64 {
    view.accounts
        .iter()
        .find(|a| a.account.id() == id)
        .expect("balance")
        .balance
        .minor()
}

fn deletion(app: &mut Application, id: AccountId) -> AccountDeletion {
    match app
        .execute(Command::PreviewDeleteAccount(id))
        .expect("deletion preview")
    {
        Response::AccountDeletion(plan) => plan,
        _ => panic!("deletion response"),
    }
}

#[test]
fn account_deletion_removes_complete_transfers_reversals_and_reopens_cleanly() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("ledger.sqlite3");
    let mut application = app(SqliteLedger::open(&path).expect("open"));
    let cash = account(&mut application, "cash", AccountKind::Cash, "1000");
    let bank = account(&mut application, "bank", AccountKind::Bank, "500");
    let card = account(&mut application, "card", AccountKind::CreditCard, "50");
    record(
        &mut application,
        TransactionKind::Income,
        cash,
        "100",
        Some(Category::Salary),
        None,
    );
    let expense = record(
        &mut application,
        TransactionKind::Expense,
        cash,
        "20",
        Some(Category::Food),
        None,
    );
    application
        .execute(Command::Reverse(expense))
        .expect("reverse expense");
    record(
        &mut application,
        TransactionKind::Transfer,
        cash,
        "200",
        None,
        Some(bank),
    );
    record(
        &mut application,
        TransactionKind::Transfer,
        bank,
        "30",
        None,
        Some(cash),
    );
    let reversed_transfer = record(
        &mut application,
        TransactionKind::Transfer,
        cash,
        "10",
        None,
        Some(card),
    );
    application
        .execute(Command::Reverse(reversed_transfer))
        .expect("reverse transfer");
    let retained = record(
        &mut application,
        TransactionKind::Expense,
        bank,
        "40",
        Some(Category::Snacks),
        None,
    );
    let before = snapshot(&mut application);
    let kept: Vec<_> = before
        .entries
        .iter()
        .filter(|entry| {
            !entry
                .postings()
                .iter()
                .any(|p| p.target() == PostingTarget::Account(cash))
        })
        .cloned()
        .collect();
    let plan = deletion(&mut application, cash);
    assert_eq!(plan.transaction_count(), 5);
    assert_eq!(plan.transfer_count(), 3);
    assert_eq!(plan.reversal_count(), 2);
    assert_eq!(plan.entry_ids().len(), 8, "includes opening and reversals");
    assert_eq!(snapshot(&mut application), before, "preview never writes");
    let bank_impact = plan
        .affected_accounts()
        .iter()
        .find(|a| a.account.id() == bank)
        .expect("bank impact");
    assert_eq!(bank_impact.before.minor(), 63_000);
    assert_eq!(bank_impact.after.minor(), 46_000);
    let card_impact = plan
        .affected_accounts()
        .iter()
        .find(|a| a.account.id() == card)
        .expect("card impact");
    assert_eq!(
        card_impact.before, card_impact.after,
        "cancelled transfer has no net effect"
    );
    assert!(
        matches!(application.execute(Command::DeleteAccount(plan)).expect("delete"), Response::AccountDeleted(id) if id == cash)
    );
    let after = snapshot(&mut application);
    assert_eq!(after.accounts.len(), 2);
    assert_eq!(after.entries, kept, "other entries preserved exactly");
    assert!(after.entries.iter().any(|e| e.id() == retained));
    assert_eq!(minor(&after, bank), 46_000);
    assert_eq!(minor(&after, card), -5_000);
    assert_eq!(after.expenses.minor(), 4_000);
    assert_eq!(after.income, Money::ZERO);
    assert_eq!(after.net_worth.minor(), 41_000);
    assert!(after.reversed.is_empty());
    drop(application);
    let mut reopened = app(SqliteLedger::open(&path).expect("reopen"));
    assert_eq!(snapshot(&mut reopened), after);
    let raw = Connection::open(&path).expect("inspect");
    assert_eq!(
        raw.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .expect("FK check"),
        0
    );
    assert_eq!(
        raw.query_row("SELECT COUNT(*) FROM postings", [], |r| r.get::<_, i64>(0))
            .expect("postings"),
        6
    );
}

#[test]
fn deleting_last_account_is_empty_and_repeated_confirmation_cannot_delete_a_replacement() {
    let mut application = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut application, "cash", AccountKind::Cash, "10");
    let plan = deletion(&mut application, cash);
    assert_eq!(plan.transaction_count(), 0);
    application
        .execute(Command::DeleteAccount(plan.clone()))
        .expect("delete");
    let empty = snapshot(&mut application);
    assert!(empty.accounts.is_empty() && empty.entries.is_empty());
    assert_eq!(empty.net_worth, Money::ZERO);
    assert!(matches!(
        application.execute(Command::DeleteAccount(plan.clone())),
        Err(AppError::Storage(StorageError::DeletionChanged))
    ));
    let replacement = account(&mut application, "cash", AccountKind::Cash, "25");
    assert_ne!(replacement, cash);
    assert!(application.execute(Command::DeleteAccount(plan)).is_err());
    assert_eq!(minor(&snapshot(&mut application), replacement), 2_500);
    assert!(
        application
            .execute(Command::PreviewDeleteAccount(cash))
            .is_err()
    );
}

#[test]
fn deletion_rechecks_concurrent_entries_and_counterparty_balance_under_write_lock() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("ledger.sqlite3");
    let mut first = app(SqliteLedger::open(&path).expect("open"));
    let cash = account(&mut first, "cash", AccountKind::Cash, "1000");
    let bank = account(&mut first, "bank", AccountKind::Bank, "500");
    record(
        &mut first,
        TransactionKind::Transfer,
        cash,
        "100",
        None,
        Some(bank),
    );
    let mut second = app(SqliteLedger::open(&path).expect("second connection"));
    for target in [cash, bank] {
        let old = deletion(&mut first, cash);
        record(
            &mut second,
            TransactionKind::Expense,
            target,
            "1",
            Some(Category::Food),
            None,
        );
        let current = snapshot(&mut second);
        assert_eq!(
            first
                .execute(Command::DeleteAccount(old))
                .expect_err("stale plan"),
            AppError::Storage(StorageError::DeletionChanged)
        );
        assert_eq!(snapshot(&mut first), current, "no deletion on stale review");
    }
    let fresh = deletion(&mut first, cash);
    first
        .execute(Command::DeleteAccount(fresh))
        .expect("fresh review succeeds");
    assert_eq!(minor(&snapshot(&mut second), bank), 49_900);
}

#[test]
fn failure_after_deleting_postings_and_entries_rolls_back_every_row() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("ledger.sqlite3");
    let mut application = app(SqliteLedger::open(&path).expect("open"));
    let cash = account(&mut application, "cash", AccountKind::Cash, "1000");
    let bank = account(&mut application, "bank", AccountKind::Bank, "0");
    let transfer = record(
        &mut application,
        TransactionKind::Transfer,
        cash,
        "10",
        None,
        Some(bank),
    );
    application
        .execute(Command::Reverse(transfer))
        .expect("reverse");
    let before = snapshot(&mut application);
    let plan = deletion(&mut application, cash);
    let raw = Connection::open(&path).expect("fault connection");
    raw.execute_batch("CREATE TRIGGER fail_account_delete BEFORE DELETE ON accounts BEGIN SELECT RAISE(ABORT, 'injected deletion failure'); END;").expect("fault trigger");
    assert!(
        application
            .execute(Command::DeleteAccount(plan.clone()))
            .is_err()
    );
    assert_eq!(snapshot(&mut application), before);
    drop(application);
    let mut reopened = app(SqliteLedger::open(&path).expect("reopen"));
    assert_eq!(snapshot(&mut reopened), before, "rollback is durable");
    raw.execute_batch("DROP TRIGGER fail_account_delete")
        .expect("remove fault");
    reopened
        .execute(Command::DeleteAccount(plan))
        .expect("retry unchanged review");
    assert_eq!(snapshot(&mut reopened).accounts.len(), 1);
}

#[test]
fn deleted_account_rejects_old_drafts_and_retries_of_deleted_transfers() {
    let mut application = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut application, "cash", AccountKind::Cash, "1000");
    let bank = account(&mut application, "bank", AccountKind::Bank, "0");
    let expense = preview(
        &mut application,
        TransactionKind::Expense,
        cash,
        "5",
        Some(Category::Food),
        None,
    );
    let transfer = preview(
        &mut application,
        TransactionKind::Transfer,
        bank,
        "1",
        None,
        Some(cash),
    );
    application
        .execute(Command::Commit(transfer.clone()))
        .expect("commit transfer");
    let plan = deletion(&mut application, cash);
    application
        .execute(Command::DeleteAccount(plan))
        .expect("delete");
    let before = snapshot(&mut application);
    for prepared in [expense, transfer] {
        assert!(application.execute(Command::Commit(prepared)).is_err());
    }
    assert_eq!(snapshot(&mut application), before);
}

#[test]
fn deletion_cannot_leave_a_remaining_balance_outside_money_bounds() {
    let mut application = app(SqliteLedger::in_memory().expect("db"));
    let bank = account(&mut application, "bank", AccountKind::Bank, "90000000000");
    let cash = account(&mut application, "cash", AccountKind::Cash, "-100");
    record(
        &mut application,
        TransactionKind::Transfer,
        bank,
        "100",
        None,
        Some(cash),
    );
    record(
        &mut application,
        TransactionKind::Income,
        bank,
        "100",
        Some(Category::Salary),
        None,
    );
    let before = snapshot(&mut application);
    assert_eq!(
        application
            .execute(Command::PreviewDeleteAccount(cash))
            .expect_err("overflow"),
        AppError::Storage(StorageError::Rule(DomainError::MoneyOverflow))
    );
    assert_eq!(snapshot(&mut application), before);
}

#[test]
fn real_file_reopens_with_exact_balances_and_no_opening_income() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("ledger.sqlite3");
    let mut first = app(SqliteLedger::open(&path).expect("open"));
    let cash = account(&mut first, "เงินสด", AccountKind::Cash, "1000");
    record(
        &mut first,
        TransactionKind::Expense,
        cash,
        "80.25",
        Some(Category::Food),
        None,
    );
    let before = snapshot(&mut first);
    assert_eq!(minor(&before, cash), 91_975);
    assert_eq!(before.income, Money::ZERO);
    assert_eq!(before.expenses.minor(), 8_025);
    drop(first);
    let mut reopened = app(SqliteLedger::open(&path).expect("reopen"));
    assert_eq!(snapshot(&mut reopened), before);
}

#[test]
fn receipt_preview_rejects_mismatches_and_persists_all_verified_bullets() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("ledger.sqlite3");
    let mut application = app(SqliteLedger::open(&path).expect("open"));
    let cash = account(&mut application, "เงินสด", AccountKind::Cash, "1000");
    let today = "2026-09-20".parse().expect("date");
    let analysis =
        analyze_receipt("กาแฟ 80.00\nอาหาร 70.00\nยอดสุทธิ 160.50", today).expect("analysis");
    let mut input = analysis.draft(today);
    input.account = Some(cash);
    input.category = Some(Category::Food);
    input.receipt.as_mut().expect("receipt").reviewed = true;
    assert!(!input.is_complete());
    assert!(
        application
            .execute(Command::Preview(input.clone()))
            .is_err()
    );
    assert_eq!(
        snapshot(&mut application).entries.len(),
        1,
        "only the opening entry exists"
    );

    let receipt = input.receipt.as_mut().expect("receipt");
    receipt.lines.push(ReceiptLineInput {
        description: "VAT 7%".into(),
        amount: "10.50".into(),
        kind: Some(ReceiptLineKind::AddedTax),
    });
    receipt.reviewed = false;
    assert!(
        application
            .execute(Command::Preview(input.clone()))
            .is_err(),
        "review cannot be bypassed through the service"
    );
    input.receipt.as_mut().expect("receipt").reviewed = true;
    let prepared = match application
        .execute(Command::Preview(input))
        .expect("balanced preview")
    {
        Response::Prepared(prepared) => prepared,
        _ => panic!("preview"),
    };
    let expected = "รายการจากใบเสร็จ\n• กาแฟ — 80.00 บาท\n• อาหาร — 70.00 บาท\n• VAT 7% — 10.50 บาท\nยอดสุทธิ 160.50 บาท";
    assert_eq!(prepared.entry.note().as_str(), expected);
    let id = prepared.entry.id();
    application
        .execute(Command::Commit(prepared))
        .expect("commit");
    drop(application);
    let mut reopened = app(SqliteLedger::open(&path).expect("reopen"));
    let view = snapshot(&mut reopened);
    assert_eq!(minor(&view, cash), 83_950);
    assert_eq!(
        view.entries
            .iter()
            .find(|entry| entry.id() == id)
            .expect("saved receipt")
            .note()
            .as_str(),
        expected
    );
    assert!(
        export_csv(&view, ExportOptions::default())
            .expect("export")
            .contents
            .contains(expected),
        "CSV retains the complete multiline receipt note"
    );
}

#[test]
fn credit_purchase_then_bank_payment_counts_expense_once() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let bank = account(&mut app, "ธนาคาร", AccountKind::Bank, "1000");
    let card = account(&mut app, "บัตร", AccountKind::CreditCard, "0");
    record(
        &mut app,
        TransactionKind::Expense,
        card,
        "100",
        Some(Category::Food),
        None,
    );
    let view = snapshot(&mut app);
    assert_eq!(view.liabilities.minor(), 10_000);
    assert_eq!(view.net_worth.minor(), 90_000);
    record(
        &mut app,
        TransactionKind::Transfer,
        bank,
        "100",
        None,
        Some(card),
    );
    let view = snapshot(&mut app);
    assert_eq!(minor(&view, bank), 90_000);
    assert_eq!(minor(&view, card), 0);
    assert_eq!(view.expenses.minor(), 10_000);
    assert_eq!(view.income.minor(), 0);
    assert_eq!(view.liabilities.minor(), 0);
}

#[test]
fn transfer_preserves_net_worth_and_opening_credit_is_debt() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let bank = account(&mut app, "bank", AccountKind::Bank, "1000");
    let card = account(&mut app, "card", AccountKind::CreditCard, "500");
    let before = snapshot(&mut app);
    assert_eq!(before.net_worth.minor(), 50_000);
    record(
        &mut app,
        TransactionKind::Transfer,
        bank,
        "200",
        None,
        Some(card),
    );
    let after = snapshot(&mut app);
    assert_eq!(after.net_worth, before.net_worth);
    assert_eq!(after.income, Money::ZERO);
    assert_eq!(after.expenses, Money::ZERO);
}

#[test]
fn retry_is_idempotent_but_a_new_identical_submission_is_a_new_expense() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let prepared = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "80",
        Some(Category::Food),
        None,
    );
    let id = prepared.entry.id();
    assert!(
        matches!(app.execute(Command::Commit(prepared.clone())).expect("first"), Response::Committed(CommitOutcome::Saved(saved)) if saved == id)
    );
    assert!(
        matches!(app.execute(Command::Commit(prepared)).expect("retry"), Response::Committed(CommitOutcome::AlreadySaved(saved)) if saved == id)
    );
    record(
        &mut app,
        TransactionKind::Expense,
        cash,
        "80",
        Some(Category::Food),
        None,
    );
    assert_eq!(snapshot(&mut app).expenses.minor(), 16_000);
}

#[test]
fn reusing_submission_id_for_different_contents_is_rejected() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let first = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "80",
        Some(Category::Food),
        None,
    );
    let mut second = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "90",
        Some(Category::Food),
        None,
    );
    second.submission = first.submission;
    app.execute(Command::Commit(first)).expect("first");
    assert!(matches!(
        app.execute(Command::Commit(second)),
        Err(AppError::Storage(StorageError::SubmissionConflict))
    ));
    assert_eq!(snapshot(&mut app).expenses.minor(), 8_000);
}

#[test]
fn cancellation_restores_balances_and_reports_and_cannot_run_twice() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let id = record(
        &mut app,
        TransactionKind::Expense,
        cash,
        "80",
        Some(Category::Food),
        None,
    );
    app.execute(Command::Reverse(id)).expect("reverse");
    let view = snapshot(&mut app);
    assert_eq!(view.expenses, Money::ZERO);
    assert_eq!(minor(&view, cash), 100_000);
    assert!(view.reversed.contains(&id));
    assert!(app.execute(Command::Reverse(id)).is_err());
    assert_eq!(snapshot(&mut app), view);
}

#[test]
fn second_posting_failure_rolls_back_the_entire_entry() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = app(SqliteLedger::open(&path).expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let before = snapshot(&mut app);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER injected_failure BEFORE INSERT ON postings WHEN NEW.ordinal=1 AND NEW.system_book='expense' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").expect("fault injection");
    let prepared = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "80",
        Some(Category::Food),
        None,
    );
    assert!(app.execute(Command::Commit(prepared)).is_err());
    assert_eq!(snapshot(&mut app), before);
    assert_eq!(
        raw.query_row("SELECT COUNT(*) FROM journal_entries", [], |r| r
            .get::<_, i64>(0))
            .expect("count"),
        1
    );
    assert_eq!(
        raw.query_row("SELECT COUNT(*) FROM postings", [], |r| r.get::<_, i64>(0))
            .expect("count"),
        2
    );
}

#[test]
fn opening_failure_rolls_back_account_creation() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = app(SqliteLedger::open(&path).expect("db"));
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER injected_failure BEFORE INSERT ON postings WHEN NEW.ordinal=1 BEGIN SELECT RAISE(ABORT,'failure'); END;").expect("inject");
    assert!(
        app.execute(Command::CreateAccount {
            name: "cash".into(),
            kind: AccountKind::Cash,
            opening: "1000".into()
        })
        .is_err()
    );
    assert!(snapshot(&mut app).accounts.is_empty());
}

#[test]
fn stale_archived_account_is_rechecked_inside_commit() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = app(SqliteLedger::open(&path).expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let prepared = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "80",
        Some(Category::Food),
        None,
    );
    Connection::open(&path)
        .expect("raw")
        .execute(
            "UPDATE accounts SET archived=1 WHERE id=?1",
            [cash.to_string()],
        )
        .expect("archive externally");
    assert!(matches!(
        app.execute(Command::Commit(prepared)),
        Err(AppError::Storage(StorageError::Rule(
            DomainError::AccountUnavailable
        )))
    ));
    assert_eq!(snapshot(&mut app).expenses, Money::ZERO);
}

#[test]
fn arithmetic_overflow_does_not_poison_the_ledger() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "90000000000");
    let before = snapshot(&mut app);
    let prepared = preview(
        &mut app,
        TransactionKind::Income,
        cash,
        "0.01",
        Some(Category::Salary),
        None,
    );
    assert!(app.execute(Command::Commit(prepared)).is_err());
    assert_eq!(snapshot(&mut app), before);
}

#[test]
fn two_connections_cannot_race_past_the_account_limit() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut setup = app(SqliteLedger::open(&path).expect("db"));
    for index in 0..99 {
        account(
            &mut setup,
            &format!("account {index}"),
            AccountKind::Cash,
            "0",
        );
    }
    drop(setup);
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for index in 0..2 {
        let path = path.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(std::thread::spawn(move || {
            let mut app = app(SqliteLedger::open(&path).expect("open concurrent"));
            barrier.wait();
            app.execute(Command::CreateAccount {
                name: format!("concurrent {index}"),
                kind: AccountKind::Cash,
                opening: "0".into(),
            })
            .is_ok()
        }));
    }
    let succeeded = handles
        .into_iter()
        .map(|handle| handle.join().expect("join") as usize)
        .sum::<usize>();
    assert_eq!(succeeded, 1);
    assert_eq!(
        snapshot(&mut app(SqliteLedger::open(&path).expect("reopen")))
            .accounts
            .len(),
        100
    );
}

#[test]
fn tampered_postings_are_detected_and_newer_databases_are_not_downgraded() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut application = app(SqliteLedger::open(&path).expect("db"));
    account(&mut application, "cash", AccountKind::Cash, "1000");
    let raw = Connection::open(&path).expect("raw");
    raw.execute("UPDATE postings SET amount_minor=3 WHERE ordinal=0", [])
        .expect("tamper");
    assert!(matches!(
        application.execute(Command::Load),
        Err(AppError::Storage(StorageError::Corrupt))
    ));
    drop(application);
    raw.pragma_update(None, "user_version", 99)
        .expect("future schema");
    assert!(matches!(
        SqliteLedger::open(&path),
        Err(StorageError::NewerDatabase)
    ));
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .expect("version"),
        99
    );
}

#[test]
fn foreign_databases_are_rejected_without_destroying_their_data() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("foreign.db");
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch(
        "CREATE TABLE other_app(secret TEXT); INSERT INTO other_app VALUES('keep me');",
    )
    .expect("foreign data");
    assert!(matches!(
        SqliteLedger::open(&path),
        Err(StorageError::Corrupt)
    ));
    assert_eq!(
        raw.query_row("SELECT secret FROM other_app", [], |r| r
            .get::<_, String>(0))
            .expect("preserved"),
        "keep me"
    );
}

#[test]
fn csv_quotes_thai_notes_newlines_and_neutralizes_spreadsheet_formulas() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut app, "=malicious()", AccountKind::Cash, "1000");
    let input = EntryInput {
        kind: TransactionKind::Expense,
        amount: "80".into(),
        account: Some(cash),
        destination: None,
        category: Some(Category::Food),
        date: "2026-09-20".into(),
        note: "=HYPERLINK(\"example\")\nข้าว,ชา".into(),
        receipt: None,
    };
    let Response::Prepared(prepared) = app.execute(Command::Preview(input)).expect("preview")
    else {
        panic!("prepared");
    };
    app.execute(Command::Commit(prepared)).expect("commit");
    let Response::Csv(file) = app
        .execute(Command::ExportCsv(ExportOptions::default()))
        .expect("csv")
    else {
        panic!("csv");
    };
    let csv = file.contents;
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("\"'=malicious()\""));
    assert!(csv.contains("\"'=HYPERLINK(\"\"example\"\")\nข้าว,ชา\""));
}

#[test]
fn users_prompt_previews_without_writing_then_commits_both_accounts_once() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let cash = account(&mut app, "เงินสด", AccountKind::Cash, "1000");
    let bank = account(&mut app, "กรุงไทย", AccountKind::Bank, "0");
    let before = snapshot(&mut app);
    let Response::Resolved(QuickResolution::Batch { mut drafts, .. }) = app
        .execute(Command::Resolve(
            "ซื้อไก่ทอดไป 30 บาท และได้เงินจาก Facebook 400 บาท บันทึกลงเงินสด และ กรุงไทยตามลำดับ".into(),
        ))
        .expect("resolve")
    else {
        panic!("batch")
    };
    assert!(
        app.execute(Command::PreviewBatch(
            drafts.iter().map(|d| d.input.clone()).collect()
        ))
        .is_err()
    );
    assert_eq!(snapshot(&mut app), before);
    drafts[0].input.category = Some(Category::Food);
    drafts[1].input.category = Some(Category::OtherIncome);
    let Response::PreparedBatch(prepared) = app
        .execute(Command::PreviewBatch(
            drafts.into_iter().map(|d| d.input).collect(),
        ))
        .expect("preview")
    else {
        panic!("prepared batch")
    };
    assert_eq!(snapshot(&mut app), before);
    app.execute(Command::CommitBatch(prepared.clone()))
        .expect("commit all");
    let after = snapshot(&mut app);
    assert_eq!(minor(&after, cash), 97_000);
    assert_eq!(minor(&after, bank), 40_000);
    assert_eq!(after.expenses.minor(), 3000);
    assert_eq!(after.income.minor(), 40000);
    assert_eq!(
        monthly_cashflow(&after).expect("flow")[5].rate(),
        Some(8604)
    );
    let Response::CommittedBatch(outcomes) =
        app.execute(Command::CommitBatch(prepared)).expect("retry")
    else {
        panic!("batch result")
    };
    assert!(
        outcomes
            .iter()
            .all(|o| matches!(o, CommitOutcome::AlreadySaved(_)))
    );
    assert_eq!(snapshot(&mut app), after);
}

#[test]
fn failure_in_second_entry_rolls_back_first_and_retry_saves_all() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = app(SqliteLedger::open(&path).expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let before = snapshot(&mut app);
    let first = preview(
        &mut app,
        TransactionKind::Income,
        cash,
        "400",
        Some(Category::OtherIncome),
        None,
    );
    let second = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "30",
        Some(Category::Food),
        None,
    );
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER batch_failure BEFORE INSERT ON postings WHEN NEW.system_book='expense' BEGIN SELECT RAISE(ABORT,'failure'); END;").expect("inject");
    assert!(
        app.execute(Command::CommitBatch(vec![first.clone(), second.clone()]))
            .is_err()
    );
    assert_eq!(snapshot(&mut app), before);
    raw.execute_batch("DROP TRIGGER batch_failure;")
        .expect("remove fault");
    app.execute(Command::CommitBatch(vec![first, second]))
        .expect("retry all");
    assert_eq!(minor(&snapshot(&mut app), cash), 137000);
}

#[test]
fn batch_revalidates_stale_accounts_and_duplicate_submission_ids_atomically() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = app(SqliteLedger::open(&path).expect("db"));
    let cash = account(&mut app, "cash", AccountKind::Cash, "1000");
    let bank = account(&mut app, "bank", AccountKind::Bank, "1000");
    let first = preview(
        &mut app,
        TransactionKind::Expense,
        cash,
        "30",
        Some(Category::Food),
        None,
    );
    let second = preview(
        &mut app,
        TransactionKind::Expense,
        bank,
        "30",
        Some(Category::Food),
        None,
    );
    let before = snapshot(&mut app);
    assert!(
        app.execute(Command::CommitBatch(vec![first.clone(), first.clone()]))
            .is_err()
    );
    assert_eq!(snapshot(&mut app), before);
    Connection::open(&path)
        .expect("raw")
        .execute(
            "UPDATE accounts SET archived=1 WHERE id=?1",
            [bank.to_string()],
        )
        .expect("archive");
    let archived = snapshot(&mut app);
    assert!(
        app.execute(Command::CommitBatch(vec![first, second]))
            .is_err()
    );
    assert_eq!(snapshot(&mut app), archived);
}
