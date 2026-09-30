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
    match app.execute(Command::Load).expect("load") {
        Response::Dashboard(v) => v,
        _ => panic!("view"),
    }
}
fn setup(repo: SqliteLedger) -> (App, AccountId) {
    let mut app = app(repo);
    app.execute(Command::CreateAccount {
        credit_cycle: None,
        name: "เงินสด".into(),
        kind: AccountKind::Cash,
        opening: "10000".into(),
    })
    .expect("account");
    let id = view(&mut app).accounts[0].account.id();
    (app, id)
}
fn input(total: &str) -> ReceivableInput {
    ReceivableInput {
        debtor: "สมชาย".into(),
        description: "ยืมซื้อคอมพิวเตอร์".into(),
        total: total.into(),
        opened: "2026-07-01".into(),
        start: "2026-07".into(),
        day: Some("15".into()),
        installments: Some("3".into()),
        source: None,
    }
}
fn preview(app: &mut App, input: ReceivableInput) -> PreparedReceivable {
    match app
        .execute(Command::PreviewReceivable(input))
        .expect("preview loan")
    {
        Response::ReceivableReview(ReceivableReview::New(p)) => *p,
        _ => panic!("review"),
    }
}

fn edit_preview(app: &mut App, id: ReceivableId, input: ReceivableInput) -> PreparedReceivableEdit {
    let current = view(app);
    let expected = current
        .receivables
        .iter()
        .find(|r| r.id() == id)
        .expect("loan")
        .clone();
    let original = current.entries.iter().find(|e| matches!(e.kind(), EntryKind::ReceivableOpening { receivable, .. } | EntryKind::Lending { receivable, .. } if *receivable == id)).expect("opening").clone();
    match app
        .execute(Command::PreviewReceivableEdit {
            expected,
            original,
            input,
        })
        .expect("preview edit")
    {
        Response::ReceivableReview(ReceivableReview::Edit(p)) => *p,
        _ => panic!("edit review"),
    }
}

fn save_edit(app: &mut App, edit: PreparedReceivableEdit) {
    assert!(matches!(
        app.execute(Command::EditReceivable(Box::new(edit)))
            .expect("edit"),
        Response::ReceivableChanged
    ));
}

#[test]
fn correct_existing_debt_updates_only_the_selected_loan_without_creating_income_or_cash() {
    let (mut app, _) = setup(SqliteLedger::in_memory().expect("db"));
    let first = create(&mut app, input("100"));
    let other = create(&mut app, input("700"));
    let before = view(&mut app);
    let mut changed = input("250.25");
    changed.debtor = "Synthetic corrected debtor".into();
    changed.description = "Synthetic corrected purpose".into();
    changed.opened = "2026-08-02".into();
    changed.start = "2026-09".into();
    changed.day = Some("30".into());
    changed.installments = Some("5".into());
    let edit = edit_preview(&mut app, first.loan.id(), changed);
    assert_eq!(
        view(&mut app),
        before,
        "preview and abandoning it must not write"
    );
    save_edit(&mut app, edit.clone());
    let after = view(&mut app);
    assert_eq!(after.accounts, before.accounts);
    assert_eq!(after.income, before.income);
    assert_eq!(after.expenses, before.expenses);
    assert_eq!(after.entries.len(), before.entries.len());
    assert!(after.receivables.contains(&other.loan));
    assert!(after.entries.contains(&other.opening.entry));
    assert!(after.receivables.contains(edit.loan()));
    assert!(after.entries.contains(edit.opening()));
    assert_eq!(summary(&mut app).outstanding.to_string(), "950.25");
    assert!(
        app.execute(Command::EditReceivable(Box::new(edit)))
            .is_err(),
        "stale double submit"
    );
    assert_eq!(view(&mut app), after);
}

#[test]
fn correcting_a_partial_loan_can_change_source_without_changing_payment_or_interest_history() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    app.execute(Command::CreateAccount {
        name: "Synthetic second account".into(),
        kind: AccountKind::Bank,
        opening: "2000".into(),
        credit_cycle: None,
    })
    .expect("second account");
    let second = view(&mut app)
        .accounts
        .iter()
        .find(|a| a.account.id() != cash)
        .expect("second")
        .account
        .id();
    let mut original = input("900");
    original.source = Some(cash);
    let loan = create(&mut app, original).loan;
    let payments = pay(&mut app, &loan, cash, "300", "10");
    let mut corrected = input("1200");
    corrected.source = Some(second);
    corrected.opened = "2026-06-20".into();
    corrected.day = Some("28".into());
    let edit = edit_preview(&mut app, loan.id(), corrected);
    save_edit(&mut app, edit);
    let after = view(&mut app);
    for payment in &payments {
        assert!(after.entries.contains(&payment.entry));
    }
    assert_eq!(after.income.to_string(), "10.00");
    assert_eq!(
        after
            .accounts
            .iter()
            .find(|a| a.account.id() == cash)
            .expect("cash")
            .balance
            .to_string(),
        "10310.00"
    );
    assert_eq!(
        after
            .accounts
            .iter()
            .find(|a| a.account.id() == second)
            .expect("second")
            .balance
            .to_string(),
        "800.00"
    );
    assert_eq!(summary(&mut app).outstanding.to_string(), "900.00");
    app.execute(Command::Reverse(payments[0].entry.id()))
        .expect("cancel principal after edit");
    assert_eq!(summary(&mut app).outstanding.to_string(), "1200.00");
    assert_eq!(view(&mut app).income.to_string(), "10.00");
}

#[test]
fn invalid_corrections_preserve_the_original_ledger() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let created = create(&mut app, input("900"));
    pay(&mut app, &created.loan, cash, "300", "0");
    let before = view(&mut app);
    for n in 0..8 {
        let mut changed = input("900");
        match n {
            0 => changed.total = "299.99".into(),
            1 => changed.opened = "2026-09-25".into(),
            2 => changed.debtor.clear(),
            3 => changed.description.clear(),
            4 => changed.day = Some("32".into()),
            5 => changed.installments = Some("0".into()),
            6 => changed.start = "2026-06".into(),
            _ => changed.source = Some("00000000-0000-0000-0000-000000000999".parse().expect("id")),
        }
        assert!(
            app.execute(Command::PreviewReceivableEdit {
                expected: created.loan.clone(),
                original: created.opening.entry.clone(),
                input: changed
            })
            .is_err(),
            "case {n}"
        );
        assert_eq!(view(&mut app), before);
    }
}

#[test]
fn concurrent_repayment_or_cancellation_invalidates_the_reviewed_correction() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("test.sqlite");
    let (mut first, cash) = setup(SqliteLedger::open(&path).expect("db"));
    let loan = create(&mut first, input("900")).loan;
    let stale = edit_preview(&mut first, loan.id(), input("1000"));
    let mut second = app(SqliteLedger::open(&path).expect("db2"));
    let payment = pay(&mut second, &loan, cash, "100", "0");
    let before = view(&mut second);
    assert!(matches!(
        first.execute(Command::EditReceivable(Box::new(stale))),
        Err(AppError::Storage(StorageError::ReceivableChanged))
    ));
    assert_eq!(view(&mut first), before);
    let stale = edit_preview(&mut first, loan.id(), input("1000"));
    second
        .execute(Command::Reverse(payment[0].entry.id()))
        .expect("cancel");
    let before = view(&mut second);
    assert!(
        first
            .execute(Command::EditReceivable(Box::new(stale)))
            .is_err()
    );
    assert_eq!(view(&mut first), before);
}

#[test]
fn source_only_edits_invalidate_old_forms_even_when_loan_metadata_is_unchanged() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let created = create(&mut app, input("900"));
    let mut changed = input("900");
    changed.source = Some(cash);
    let edit = edit_preview(&mut app, created.loan.id(), changed);
    save_edit(&mut app, edit);
    assert!(matches!(
        app.execute(Command::PreviewReceivableEdit {
            expected: created.loan,
            original: created.opening.entry,
            input: input("1000")
        }),
        Err(AppError::Storage(StorageError::ReceivableChanged))
    ));
}

#[test]
fn correction_rolls_back_on_posting_failure_and_roundtrips_through_backup_and_reopen() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("test.sqlite");
    let (mut application, cash) = setup(SqliteLedger::open(&path).expect("db"));
    let loan = create(&mut application, input("900")).loan;
    pay(&mut application, &loan, cash, "100", "5");
    let edit = edit_preview(&mut application, loan.id(), input("700"));
    let before = view(&mut application);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER fail_correction BEFORE UPDATE ON postings BEGIN SELECT RAISE(ABORT,'synthetic fault'); END;").expect("trigger");
    assert!(
        application
            .execute(Command::EditReceivable(Box::new(edit.clone())))
            .is_err()
    );
    assert_eq!(view(&mut application), before);
    raw.execute_batch("DROP TRIGGER fail_correction;")
        .expect("drop");
    save_edit(&mut application, edit);
    let expected = view(&mut application);
    drop(application);
    let mut repo = SqliteLedger::open(&path).expect("reopen");
    let bytes = repo.export_backup().expect("export");
    let mut restored = SqliteLedger::in_memory().expect("restore db");
    restored.restore_backup(&bytes).expect("restore");
    assert_eq!(view(&mut app(restored)), expected);
    assert_eq!(view(&mut app(repo)), expected);
    assert_eq!(
        raw.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .expect("integrity"),
        "ok"
    );
    assert_eq!(
        raw.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .expect("fk"),
        0
    );
}

#[test]
fn fully_paid_debt_can_be_corrected_and_cancelled_loans_cannot_be_reactivated_by_editing() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let created = create(&mut app, input("100"));
    pay(&mut app, &created.loan, cash, "100", "0");
    let mut changed = input("100");
    changed.description = "Synthetic name correction".into();
    let edit = edit_preview(&mut app, created.loan.id(), changed);
    save_edit(&mut app, edit);
    assert_eq!(summary(&mut app).items[0].status, ReceivableStatus::Paid);
    let edit = edit_preview(&mut app, created.loan.id(), input("200"));
    save_edit(&mut app, edit);
    assert_eq!(summary(&mut app).outstanding.to_string(), "100.00");
    let mut input = input("100");
    input.source = Some(cash);
    let other = create(&mut app, input.clone());
    app.execute(Command::Reverse(other.opening.entry.id()))
        .expect("cancel loan");
    let before = view(&mut app);
    assert!(
        app.execute(Command::PreviewReceivableEdit {
            expected: other.loan,
            original: other.opening.entry,
            input
        })
        .is_err()
    );
    assert_eq!(view(&mut app), before);
}
fn create(app: &mut App, input: ReceivableInput) -> PreparedReceivable {
    let p = preview(app, input);
    app.execute(Command::CreateReceivable(p.clone()))
        .expect("create loan");
    p
}
fn repay_input(
    loan: &Receivable,
    account: AccountId,
    principal: &str,
    interest: &str,
) -> RepaymentInput {
    RepaymentInput {
        receivable: loan.id(),
        account: Some(account),
        principal: principal.into(),
        interest: interest.into(),
        date: "2026-09-24".into(),
    }
}
fn repayment(
    app: &mut App,
    loan: &Receivable,
    account: AccountId,
    principal: &str,
    interest: &str,
) -> Vec<PreparedEntry> {
    match app
        .execute(Command::PreviewRepayment(repay_input(
            loan, account, principal, interest,
        )))
        .expect("preview repayment")
    {
        Response::ReceivableReview(ReceivableReview::Payment(p)) => p,
        _ => panic!("repayment"),
    }
}
fn pay(
    app: &mut App,
    loan: &Receivable,
    account: AccountId,
    principal: &str,
    interest: &str,
) -> Vec<PreparedEntry> {
    let p = repayment(app, loan, account, principal, interest);
    app.execute(Command::ReceiveRepayment(p.clone()))
        .expect("receive");
    p
}
fn summary(app: &mut App) -> ReceivableSummary {
    receivable_summary(&view(app)).expect("summary")
}

#[test]
fn existing_debt_is_an_asset_and_repayment_preserves_net_worth_and_income() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let p = preview(&mut app, input("900"));
    assert!(view(&mut app).receivables.is_empty());
    app.execute(Command::CreateReceivable(p.clone()))
        .expect("create");
    app.execute(Command::CreateReceivable(p.clone()))
        .expect("retry creation");
    let v = view(&mut app);
    assert_eq!(v.accounts[0].balance.to_string(), "10000.00");
    assert_eq!(v.assets.to_string(), "10900.00");
    assert_eq!(v.income, Money::ZERO);
    let payments = pay(&mut app, &p.loan, cash, "300", "0");
    app.execute(Command::ReceiveRepayment(payments))
        .expect("retry repayment");
    let v = view(&mut app);
    assert_eq!(v.accounts[0].balance.to_string(), "10300.00");
    assert_eq!(v.net_worth.to_string(), "10900.00");
    assert_eq!((v.income, v.expenses), (Money::ZERO, Money::ZERO));
    let s = receivable_summary(&v).expect("summary");
    assert_eq!(s.items[0].remaining_installments, Some(2));
    assert_eq!(s.outstanding.to_string(), "600.00");
    let flow = monthly_cashflow(&v).expect("flow");
    assert!(
        flow.iter()
            .all(|m| m.income == Money::ZERO && m.expenses == Money::ZERO)
    );
    for kind in AccountingReport::ALL {
        let csv = export_csv(
            &v,
            ExportOptions {
                language: ExportLanguage::English,
                report: kind,
            },
        )
        .expect("balanced export");
        assert!(
            csv.contents
                .contains(&format!("receivable:{}", p.loan.id()))
        );
        assert!(csv.contents.contains("สมชาย"));
    }
}

#[test]
fn new_lending_moves_cash_to_receivables_and_interest_is_separate_income() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let mut i = input("900");
    i.source = Some(cash);
    let p = create(&mut app, i);
    let v = view(&mut app);
    assert_eq!(v.accounts[0].balance.to_string(), "9100.00");
    assert_eq!(v.net_worth.to_string(), "10000.00");
    assert_eq!(v.expenses, Money::ZERO);
    pay(&mut app, &p.loan, cash, "300", "15");
    let v = view(&mut app);
    assert_eq!(v.accounts[0].balance.to_string(), "9415.00");
    assert_eq!(v.net_worth.to_string(), "10015.00");
    assert_eq!(v.income.to_string(), "15.00");
    assert_eq!(summary(&mut app).outstanding.to_string(), "600.00");
    assert_eq!(
        monthly_cashflow(&v)
            .expect("flow")
            .last()
            .expect("month")
            .income
            .to_string(),
        "15.00"
    );
    assert!(app.execute(Command::Reverse(p.opening.entry.id())).is_err());
    assert!(app.execute(Command::PreviewDeleteAccount(cash)).is_err());
}

#[test]
fn partial_payments_rounding_completion_and_cancellation_restore_installments() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let p = create(&mut app, input("100.01"));
    assert_eq!(
        p.loan.installment_amount(1).expect("first").to_string(),
        "33.33"
    );
    assert_eq!(
        p.loan.installment_amount(3).expect("last").to_string(),
        "33.35"
    );
    let first = pay(&mut app, &p.loan, cash, "33.32", "0");
    let s = summary(&mut app);
    assert_eq!(s.items[0].paid_installments, Some(0));
    assert_eq!(s.items[0].next_amount.expect("partial").to_string(), "0.01");
    assert_eq!(s.overdue.to_string(), "66.69");
    pay(&mut app, &p.loan, cash, "66.69", "0");
    assert_eq!(summary(&mut app).items[0].status, ReceivableStatus::Paid);
    assert_eq!(summary(&mut app).items[0].remaining_installments, Some(0));
    app.execute(Command::Reverse(first[0].entry.id()))
        .expect("cancel receipt");
    let s = summary(&mut app);
    assert_eq!(s.items[0].status, ReceivableStatus::Overdue);
    assert_eq!(s.items[0].remaining_installments, Some(1));
    assert_eq!(s.outstanding.to_string(), "33.32");
    pay(&mut app, &p.loan, cash, "33.32", "0");
    assert_eq!(summary(&mut app).items[0].status, ReceivableStatus::Paid);
}

#[test]
fn optional_day_and_count_do_not_invent_overdue_amounts_and_unlimited_still_closes() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    for (index, (count, day)) in [(None, None), (None, Some("31")), (Some("2"), None)]
        .into_iter()
        .enumerate()
    {
        let mut i = input("100");
        i.installments = count.map(str::to_string);
        i.day = day.map(str::to_string);
        i.description = format!("หนี้ {index}");
        let p = create(&mut app, i);
        let s = summary(&mut app);
        let row = s
            .items
            .iter()
            .find(|r| r.loan.id() == p.loan.id())
            .expect("row");
        assert_eq!(row.status, ReceivableStatus::Unscheduled);
        assert_eq!(row.overdue, Money::ZERO);
        if day.is_none() {
            assert!(row.next_due.is_none());
        }
        pay(&mut app, &p.loan, cash, "25", "0");
        let s = summary(&mut app);
        let row = s
            .items
            .iter()
            .find(|r| r.loan.id() == p.loan.id())
            .expect("row");
        if count.is_none() {
            assert!(row.remaining_installments.is_none());
        }
        if day.is_some() {
            assert_eq!(row.next_due.expect("next date").to_string(), "2026-10-31");
        }
        pay(&mut app, &p.loan, cash, "75", "0");
        assert_eq!(
            summary(&mut app)
                .items
                .iter()
                .find(|r| r.loan.id() == p.loan.id())
                .expect("row")
                .status,
            ReceivableStatus::Paid
        );
    }
}

#[test]
fn calendar_clamps_leap_months_and_due_today_is_not_overdue() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let mut i = input("600");
    i.opened = "2024-01-01".into();
    i.start = "2024-01".into();
    i.day = Some("31".into());
    create(&mut app, i);
    let mut v = view(&mut app);
    v.today = "2024-02-29".parse().expect("date");
    let s = receivable_summary(&v).expect("summary");
    assert_eq!(s.overdue.to_string(), "200.00");
    v.today = "2024-03-01".parse().expect("date");
    assert_eq!(
        receivable_summary(&v).expect("summary").overdue.to_string(),
        "400.00"
    );
}

#[test]
fn overpayment_missing_fields_and_invalid_dates_are_rejected_without_writes() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let p = create(&mut app, input("100"));
    let original = view(&mut app);
    for (principal, interest) in [
        ("101", "0"),
        ("0", "0"),
        ("-5", "0"),
        ("50", "-1"),
        ("abc", "0"),
    ] {
        assert!(
            app.execute(Command::PreviewRepayment(repay_input(
                &p.loan, cash, principal, interest
            )))
            .is_err()
        );
    }
    for date in ["2026-06-30", "2026-09-25", "bad"] {
        let mut i = repay_input(&p.loan, cash, "50", "0");
        i.date = date.into();
        assert!(app.execute(Command::PreviewRepayment(i)).is_err());
    }
    let mut i = repay_input(&p.loan, cash, "50", "0");
    i.account = None;
    assert!(app.execute(Command::PreviewRepayment(i)).is_err());
    assert_eq!(view(&mut app), original);
    for mutate in 0..8 {
        let mut i = input("100");
        match mutate {
            0 => i.debtor.clear(),
            1 => i.description.clear(),
            2 => i.total = "0".into(),
            3 => i.installments = Some("1201".into()),
            4 => i.day = Some("32".into()),
            5 => i.start = "2026-06".into(),
            6 => i.opened = "2026-09-25".into(),
            _ => i.total = "0.01".into(),
        }
        assert!(
            app.execute(Command::PreviewReceivable(i)).is_err(),
            "case {mutate}"
        );
    }
}

#[test]
fn concurrent_receipts_recheck_remaining_balance_under_write_lock() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite");
    let (mut first, cash) = setup(SqliteLedger::open(&path).expect("db"));
    let loan = create(&mut first, input("900")).loan;
    let stale = repayment(&mut first, &loan, cash, "700", "10");
    let mut second = app(SqliteLedger::open(&path).expect("second"));
    pay(&mut second, &loan, cash, "300", "0");
    assert!(first.execute(Command::ReceiveRepayment(stale)).is_err());
    let v = view(&mut first);
    assert_eq!(v.income, Money::ZERO);
    assert_eq!(summary(&mut first).outstanding.to_string(), "600.00");
    assert_eq!(v.accounts[0].balance.to_string(), "10300.00");
}

#[test]
fn failure_in_interest_posting_rolls_back_principal_and_can_retry() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite");
    let (mut app, cash) = setup(SqliteLedger::open(&path).expect("db"));
    let loan = create(&mut app, input("900")).loan;
    let p = repayment(&mut app, &loan, cash, "300", "10");
    let before = view(&mut app);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER fail_interest BEFORE INSERT ON postings WHEN NEW.system_book='income' BEGIN SELECT RAISE(ABORT,'fault'); END;").expect("trigger");
    assert!(app.execute(Command::ReceiveRepayment(p.clone())).is_err());
    assert_eq!(view(&mut app), before);
    raw.execute_batch("DROP TRIGGER fail_interest;")
        .expect("drop");
    app.execute(Command::ReceiveRepayment(p)).expect("retry");
    assert_eq!(summary(&mut app).outstanding.to_string(), "600.00");
    assert_eq!(view(&mut app).income.to_string(), "10.00");
}

#[test]
fn creation_rolls_back_metadata_with_failed_posting_and_reopens_after_retry() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite");
    let (mut app, _) = setup(SqliteLedger::open(&path).expect("db"));
    let p = preview(&mut app, input("900"));
    let before = view(&mut app);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER fail_loan BEFORE INSERT ON postings WHEN NEW.system_book='receivable' BEGIN SELECT RAISE(ABORT,'fault'); END;").expect("trigger");
    assert!(app.execute(Command::CreateReceivable(p.clone())).is_err());
    assert_eq!(view(&mut app), before);
    raw.execute_batch("DROP TRIGGER fail_loan;").expect("drop");
    app.execute(Command::CreateReceivable(p)).expect("retry");
    let expected = view(&mut app);
    drop(app);
    let mut reopened = LedgerApplication::new(
        SqliteLedger::open(&path).expect("reopen"),
        FixedClock,
        RandomIds,
    );
    assert_eq!(view(&mut reopened), expected);
}

#[test]
fn cancellation_of_unpaid_advance_removes_receivable_and_restores_cash() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let mut i = input("900");
    i.source = Some(cash);
    let p = create(&mut app, i);
    app.execute(Command::Reverse(p.opening.entry.id()))
        .expect("cancel loan");
    let s = summary(&mut app);
    assert_eq!(s.items[0].status, ReceivableStatus::Cancelled);
    assert_eq!(s.outstanding, Money::ZERO);
    assert_eq!(view(&mut app).accounts[0].balance.to_string(), "10000.00");
    assert!(
        app.execute(Command::PreviewRepayment(repay_input(
            &p.loan, cash, "1", "0"
        )))
        .is_err()
    );
}

#[test]
fn v3_upgrade_preserves_accounts_postings_and_recurring_plans() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite");
    let (mut first, cash) = setup(SqliteLedger::open(&path).expect("db"));
    first
        .execute(Command::AddRecurring(RecurringInput {
            name: "เช่า".into(),
            amount: "100".into(),
            day: "1".into(),
            start: "2026-09".into(),
            account: Some(cash),
            category: Some(Category::Rent),
            installments: Some("12".into()),
        }))
        .expect("plan");
    let expected = view(&mut first);
    drop(first);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch(
        "DROP TABLE crypto_holding_changes; DROP TABLE crypto_prices; ALTER TABLE accounts DROP COLUMN sol_atoms; ALTER TABLE accounts DROP COLUMN btc_atoms; ALTER TABLE accounts DROP COLUMN payment_day; ALTER TABLE accounts DROP COLUMN closing_day; DROP TABLE user_preferences; DROP TRIGGER lock_currency_accounts; DROP TRIGGER lock_currency_recurring; DROP TRIGGER lock_currency_receivables; DROP TABLE ledger_settings; DROP TABLE prompt_submissions; DROP TABLE receivables; PRAGMA user_version=3;",
    )
    .expect("v3");
    let mut migrated = app(SqliteLedger::open(&path).expect("migrate"));
    assert_eq!(view(&mut migrated), expected);
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .expect("version"),
        10
    );
    assert_eq!(
        raw.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .expect("fk"),
        0
    );
    create(&mut migrated, input("900"));
    assert_eq!(summary(&mut migrated).total.to_string(), "900.00");
}

#[test]
fn cumulative_principal_bounds_are_checked_even_after_old_loans_are_repaid() {
    let mut app = app(SqliteLedger::in_memory().expect("db"));
    let maximum = Money::from_minor(Money::MAX_MINOR)
        .expect("maximum")
        .to_string();
    app.execute(Command::CreateAccount {
        credit_cycle: None,
        name: "เงินสด".into(),
        kind: AccountKind::Cash,
        opening: maximum.clone(),
    })
    .expect("account");
    let account = view(&mut app).accounts[0].account.id();
    let mut loan = input(&maximum);
    loan.source = Some(account);
    let first = create(&mut app, loan);
    pay(&mut app, &first.loan, account, &maximum, "0");
    let before = view(&mut app);
    let mut next = input("100");
    next.source = Some(account);
    assert!(app.execute(Command::PreviewReceivable(next)).is_err());
    assert_eq!(view(&mut app), before);
    assert!(receivable_summary(&before).is_ok());
}

#[test]
fn unlimited_collection_at_calendar_end_keeps_remaining_balance_without_invalid_next_date() {
    struct LastDay;
    impl Clock for LastDay {
        fn today(&self) -> Result<EntryDate, AppError> {
            Ok("9999-12-31".parse()?)
        }
    }
    let mut app =
        LedgerApplication::new(SqliteLedger::in_memory().expect("db"), LastDay, RandomIds);
    app.execute(Command::CreateAccount {
        credit_cycle: None,
        name: "เงินสด".into(),
        kind: AccountKind::Cash,
        opening: "0".into(),
    })
    .expect("account");
    let Response::Dashboard(v) = app.execute(Command::Load).expect("view") else {
        panic!("view")
    };
    let account = v.accounts[0].account.id();
    let Response::ReceivableReview(ReceivableReview::New(p)) = app
        .execute(Command::PreviewReceivable(ReceivableInput {
            debtor: "เอ".into(),
            description: "ทดสอบปฏิทิน".into(),
            total: "100".into(),
            opened: "9999-12-01".into(),
            start: "9999-12".into(),
            day: Some("31".into()),
            installments: None,
            source: None,
        }))
        .expect("preview")
    else {
        panic!("preview")
    };
    let loan = p.loan.id();
    app.execute(Command::CreateReceivable(*p)).expect("create");
    let Response::ReceivableReview(ReceivableReview::Payment(p)) = app
        .execute(Command::PreviewRepayment(RepaymentInput {
            receivable: loan,
            account: Some(account),
            principal: "1".into(),
            interest: "0".into(),
            date: "9999-12-31".into(),
        }))
        .expect("payment")
    else {
        panic!("payment")
    };
    app.execute(Command::ReceiveRepayment(p)).expect("commit");
    let Response::Dashboard(v) = app.execute(Command::Load).expect("view") else {
        panic!("view")
    };
    let summary = receivable_summary(&v).expect("summary");
    assert_eq!(summary.outstanding.to_string(), "99.00");
    assert!(summary.items[0].next_due.is_none());
}
