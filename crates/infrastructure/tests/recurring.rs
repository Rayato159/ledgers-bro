#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{RandomIds, SqliteLedger};
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn new_plan_default_rolls_forward_only_after_the_actual_due_date() {
    for (today, day, expected) in [
        ("2026-09-25", 1, "2026-10"),
        ("2026-09-25", 25, "2026-09"),
        ("2026-09-25", 26, "2026-09"),
        ("2026-12-31", 1, "2027-01"),
        ("2026-02-28", 31, "2026-02"),
        ("2024-02-29", 31, "2024-02"),
        ("2024-02-29", 28, "2024-03"),
    ] {
        assert_eq!(
            default_recurring_month(today.parse().expect("today"), day)
                .expect("default")
                .to_string(),
            expected
        );
    }
    assert!(default_recurring_month("2026-09-25".parse().expect("today"), 0).is_err());
}

#[test]
fn editing_future_terms_preserves_paid_history_and_prevents_duplicate_obligations() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let (mut app, cash) = setup(SqliteLedger::open(&path).expect("db"));
    let old = add(&mut app, cash, "Discord", "215", "1", "2026-08");
    let august = month().shifted(-1).expect("august");
    let paid = payment(&mut app, &old, august, "215");
    app.execute(Command::PayRecurring(paid)).expect("paid");
    let before = view(&mut app);
    let mut edited = recurring_edit_input(&old, month());
    edited.name = "Discord revised".into();
    edited.amount = "250".into();
    edited.day = "31".into();
    edited.category = Some(Category::Food);
    edited.installments = Some("3".into());
    app.execute(Command::EditRecurring {
        expected: old.clone(),
        input: edited.clone(),
    })
    .expect("edit");
    let after = view(&mut app);
    assert_eq!(before.entries, after.entries);
    assert_eq!(before.settlements, after.settlements);
    assert_eq!(
        recurring_month(&before, august).expect("old month"),
        recurring_month(&after, august)
            .map(|mut v| {
                // The old entity gains a stop boundary; all period terms remain intact.
                v.items[0].schedule = old.clone();
                v
            })
            .expect("preserved month")
    );
    let september = recurring_month(&after, month()).expect("revised month");
    assert_eq!(september.items.len(), 1);
    assert_eq!(september.pending.to_string(), "250.00");
    assert_eq!(september.items[0].due.to_string(), "2026-09-30");
    assert_eq!(september.items[0].schedule.category(), Category::Food);
    assert!(
        !september.items[0]
            .schedule
            .occurs_in("2026-12".parse().expect("after last"))
    );
    assert!(
        app.execute(Command::EditRecurring {
            expected: old,
            input: edited
        })
        .is_err(),
        "stale edit cannot split twice"
    );
    drop(app);
    let mut reopened = self::app(SqliteLedger::open(&path).expect("reopen"));
    assert_eq!(view(&mut reopened), after);
}

#[test]
fn edit_is_atomic_and_rejects_paid_future_months_and_invalid_accounts() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let old = add(&mut app, cash, "Plan", "215", "1", "2026-09");
    let prepared = payment(&mut app, &old, month().shifted(1).expect("october"), "215");
    app.execute(Command::PayRecurring(prepared))
        .expect("prepaid");
    let before = view(&mut app);
    let mut edited = recurring_edit_input(&old, month());
    edited.amount = "300".into();
    assert!(matches!(
        app.execute(Command::EditRecurring {
            expected: old.clone(),
            input: edited.clone()
        }),
        Err(AppError::Storage(StorageError::RecurringEditPaid))
    ));
    assert_eq!(view(&mut app), before);
    edited.start = "2026-11".into();
    edited.account = None;
    assert!(
        app.execute(Command::EditRecurring {
            expected: old.clone(),
            input: edited.clone()
        })
        .is_err()
    );
    assert_eq!(view(&mut app), before);
    edited.account = Some(cash);
    app.execute(Command::EditRecurring {
        expected: old,
        input: edited,
    })
    .expect("edit after prepaid");
    assert_eq!(
        recurring_month(&view(&mut app), month())
            .expect("september retains old")
            .pending
            .to_string(),
        "215.00"
    );
}

#[test]
fn finite_plan_edit_defaults_to_remaining_contractual_months() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let old = add(&mut app, cash, "Three months", "100", "5", "2026-09")
        .with_installments(Some(3))
        .expect("terms");
    assert_eq!(
        recurring_edit_input(&old, month().shifted(1).expect("october"))
            .installments
            .as_deref(),
        Some("2")
    );
}

#[test]
fn edit_stopped_arrears_keeps_successor_and_paid_history_after_reopen() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let (mut app, cash) = setup(SqliteLedger::open(&path).expect("db"));
    let old = add(&mut app, cash, "Insurance", "200", "11", "2026-08");
    let august = "2026-08".parse().expect("august");
    let october = "2026-10".parse().expect("october");
    let paid = payment(&mut app, &old, august, "200");
    app.execute(Command::PayRecurring(paid)).expect("paid");
    let mut future = recurring_edit_input(&old, october);
    future.amount = "300".into();
    app.execute(Command::EditRecurring {
        expected: old.clone(),
        input: future,
    })
    .expect("successor");
    let before = view(&mut app);
    let stopped = before
        .recurring
        .iter()
        .find(|s| s.id() == old.id())
        .expect("stopped old")
        .clone();
    let mut input = recurring_edit_input(&stopped, month());
    input.amount = "250".into();
    input.name = "Updated arrears".into();
    app.execute(Command::EditRecurring {
        expected: stopped.clone(),
        input: input.clone(),
    })
    .expect("edit stopped arrears");
    let after = view(&mut app);
    assert_eq!(after.entries, before.entries);
    assert_eq!(after.settlements, before.settlements);
    let september = recurring_month(&after, month()).expect("september");
    assert_eq!(september.items.len(), 1);
    assert_eq!(september.pending.to_string(), "250.00");
    assert_eq!(september.items[0].schedule.stopped_from(), Some(october));
    for period in [october, "2026-11".parse().expect("november")] {
        assert_eq!(
            recurring_month(&before, period),
            recurring_month(&after, period),
            "no overlap with successor"
        );
    }
    assert!(
        app.execute(Command::EditRecurring {
            expected: stopped,
            input
        })
        .is_err(),
        "stale edits cannot duplicate arrears"
    );
    let revised = september.items[0].schedule.clone();
    let invalid = recurring_edit_input(&revised, october);
    assert!(
        app.execute(Command::EditRecurring {
            expected: revised.clone(),
            input: invalid
        })
        .is_err(),
        "cannot edit at the stop boundary"
    );
    let paid = payment(&mut app, &revised, month(), "250");
    app.execute(Command::PayRecurring(paid))
        .expect("pay edited arrears");
    let final_view = view(&mut app);
    assert_eq!(
        recurring_month(&final_view, month()).expect("paid").pending,
        Money::ZERO
    );
    drop(app);
    let mut reopened = self::app(SqliteLedger::open(&path).expect("reopen"));
    assert_eq!(view(&mut reopened), final_view);
}

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
    let account = view(&mut app).accounts[0].account.id();
    (app, account)
}
fn add(
    app: &mut App,
    account: AccountId,
    name: &str,
    amount: &str,
    day: &str,
    start: &str,
) -> RecurringExpense {
    app.execute(Command::AddRecurring(RecurringInput {
        installments: None,
        name: name.into(),
        amount: amount.into(),
        day: day.into(),
        start: start.into(),
        account: Some(account),
        category: Some(Category::Rent),
    }))
    .expect("schedule");
    view(app)
        .recurring
        .into_iter()
        .find(|s| s.name().as_str() == name)
        .expect("created")
}
fn month() -> Month {
    "2026-09".parse().expect("month")
}
fn payment(
    app: &mut App,
    schedule: &RecurringExpense,
    month: Month,
    amount: &str,
) -> PreparedRecurringPayment {
    let mut input = recurring_payment_input(schedule, "2026-09-24".parse().expect("today"));
    input.amount = amount.into();
    match app
        .execute(Command::PreviewRecurringPayment {
            expected: schedule.clone(),
            month,
            input,
        })
        .expect("preview")
    {
        Response::PreparedRecurringPayment(p) => p,
        _ => panic!("prepared"),
    }
}
fn record(app: &mut App, account: AccountId, amount: &str, kind: TransactionKind) -> PreparedEntry {
    let input = EntryInput {
        account: Some(account),
        amount: amount.into(),
        kind,
        category: Some(if kind == TransactionKind::Income {
            Category::Salary
        } else {
            Category::Rent
        }),
        ..EntryInput::empty("2026-09-24".parse().expect("today"))
    };
    let Response::Prepared(prepared) = app.execute(Command::Preview(input)).expect("preview")
    else {
        panic!("prepared")
    };
    app.execute(Command::Commit(prepared.clone()))
        .expect("commit");
    prepared
}
fn projection(app: &mut App) -> MonthlyFlow {
    let v = view(app);
    let actual = monthly_cashflow(&v).expect("flow").pop().expect("current");
    projected_cashflow(&v, &actual).expect("projected")
}

#[test]
fn expense_drilldown_reconciles_actual_pending_payment_and_reversal() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let expense = record(&mut app, cash, "13337.65", TransactionKind::Expense);
    let plan = add(&mut app, cash, "Pending bills", "20658.27", "11", "2026-09");
    let v = view(&mut app);
    let details = cashflow_breakdown(&v, month()).expect("breakdown");
    assert_eq!(details.recorded.expenses.to_string(), "13337.65");
    assert_eq!(details.pending_total.to_string(), "20658.27");
    assert_eq!(details.expense_entries.len(), 1);
    assert_eq!(details.pending_items.len(), 1);
    let projected = projection(&mut app);
    assert_eq!(projected.expenses.to_string(), "33995.92");
    assert_eq!(projected.rate(), Some(-10000));
    assert_eq!(
        details
            .recorded
            .expenses
            .checked_add(details.pending_total)
            .expect("sum"),
        projected.expenses
    );
    assert_eq!(
        details.recorded,
        monthly_cashflow(&v)
            .expect("recorded")
            .pop()
            .expect("current")
    );
    let prepared = payment(&mut app, &plan, month(), "20658.27");
    app.execute(Command::PayRecurring(prepared)).expect("pay");
    let paid = cashflow_breakdown(&view(&mut app), month()).expect("paid details");
    assert_eq!(paid.pending_total, Money::ZERO);
    assert!(paid.pending_items.is_empty());
    assert_eq!(
        paid.recorded.expenses, projected.expenses,
        "paid plan is not counted twice"
    );
    app.execute(Command::Reverse(expense.entry.id()))
        .expect("reverse");
    let reversed = cashflow_breakdown(&view(&mut app), month()).expect("after reversal");
    assert_eq!(reversed.recorded.expenses.to_string(), "20658.27");
    assert_eq!(reversed.expense_entries.len(), 1);
    assert!(reversed.income_entries.is_empty());
}

#[test]
fn other_expenses_are_split_by_description_and_exclude_reversals_and_future_dates() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let mut v = view(&mut app);
    for (i, note, category, date, amount) in [
        (
            1,
            "Domain renewal",
            Category::OtherExpense,
            "2026-09-24",
            "200",
        ),
        (
            2,
            "Domain renewal\n• Hosting add-on",
            Category::OtherExpense,
            "2026-09-24",
            "100",
        ),
        (
            3,
            "Drawing software",
            Category::OtherExpense,
            "2026-09-24",
            "350",
        ),
        (4, "Lunch", Category::Food, "2026-09-24", "50"),
        (5, "Cancelled", Category::OtherExpense, "2026-09-24", "500"),
        (6, "Future", Category::OtherExpense, "2026-09-30", "600"),
        (7, "Last year", Category::OtherExpense, "2025-09-24", "700"),
    ] {
        let id = format!("00000000-0000-4000-8000-{i:012}")
            .parse()
            .expect("id");
        v.entries.push(
            JournalEntry::record(
                id,
                date.parse().expect("date"),
                Note::new(note).expect("note"),
                EntryKind::Expense {
                    account: cash,
                    amount: PositiveMoney::new(amount.parse().expect("amount")).expect("positive"),
                    category,
                },
            )
            .expect("entry"),
        );
        if i == 5 {
            v.reversed.insert(id);
        }
    }
    let slices = expense_slices(&v, month()).expect("slices");
    assert_eq!(slices.len(), 3);
    assert_eq!(slices[0].description.as_deref(), Some("Drawing software"));
    assert_eq!(slices[1].amount.to_string(), "300.00");
    assert_eq!(slices[1].description.as_deref(), Some("Domain renewal"));
    let total = slices
        .iter()
        .try_fold(Money::ZERO, |s, v| s.checked_add(v.amount))
        .expect("sum");
    assert_eq!(total.to_string(), "700.00");
    assert_eq!(
        total,
        cashflow_breakdown(&v, month())
            .expect("breakdown")
            .recorded
            .expenses
    );
}

#[test]
fn large_history_monthly_lookup_preserves_totals() {
    let (mut app, cash) = setup(SqliteLedger::in_memory().expect("db"));
    let mut v = view(&mut app);
    let today = v.today;
    for i in 0..20_000 {
        v.entries.push(
            JournalEntry::record(
                format!("00000000-0000-4000-8000-{i:012}")
                    .parse()
                    .expect("id"),
                today,
                Note::new("Historical entry").expect("note"),
                EntryKind::Expense {
                    account: cash,
                    amount: PositiveMoney::new("1".parse().expect("amount")).expect("positive"),
                    category: Category::Food,
                },
            )
            .expect("entry"),
        );
    }
    for i in 0..500 {
        let id = format!("10000000-0000-4000-8000-{i:012}")
            .parse()
            .expect("id");
        v.recurring.push(
            RecurringExpense::new(
                id,
                AccountName::new(&format!("Plan {i}")).expect("name"),
                PositiveMoney::new("100".parse().expect("amount")).expect("positive"),
                Category::Rent,
                Some(cash),
                MonthlyDue::new(11, month()).expect("due"),
            )
            .expect("plan"),
        );
        if i % 2 == 0 {
            v.settlements.push(RecurringSettlement {
                recurring: id,
                month: month(),
                entry: v.entries[19_000 + i].id(),
            });
        }
    }
    let started = std::time::Instant::now();
    let current = recurring_month(&v, month()).expect("large history");
    let elapsed = started.elapsed();
    assert_eq!(current.items.len(), 500);
    assert_eq!(current.pending.to_string(), "25000.00");
    assert_eq!(current.paid.to_string(), "250.00");
    assert_eq!(
        cashflow_breakdown(&v, month())
            .expect("breakdown")
            .recorded
            .expenses
            .to_string(),
        "20000.00"
    );
    eprintln!("20,000 entries / 500 plans: monthly lookup {:?}", elapsed);
}

#[test]
fn calendar_clamps_month_ends_and_keeps_start_stop_boundaries() {
    for (month, day, expected) in [
        ("2024-02", 31, "2024-02-29"),
        ("2025-02", 30, "2025-02-28"),
        ("2026-04", 31, "2026-04-30"),
        ("2026-12", 31, "2026-12-31"),
        ("9999-12", 31, "9999-12-31"),
    ] {
        let m: Month = month.parse().expect("month");
        assert_eq!(m.on_day(day).expect("due").to_string(), expected);
    }
    assert_eq!(
        "2026-12"
            .parse::<Month>()
            .expect("month")
            .shifted(1)
            .expect("next")
            .to_string(),
        "2027-01"
    );
    assert!(month().on_day(0).is_err());
    assert!(month().on_day(32).is_err());
    assert!("2026-13".parse::<Month>().is_err());
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let s = add(&mut app, account, "ค่าเช่า", "200", "31", "2026-09");
    assert!(!s.occurs_in(month().shifted(-1).expect("previous")));
    let stopped = s
        .stop_from(month().shifted(1).expect("next"))
        .expect("stop");
    assert!(stopped.occurs_in(month()));
    assert!(!stopped.occurs_in(month().shifted(1).expect("next")));
}

#[test]
fn forecast_counts_pending_once_and_payment_retry_and_reversal_reconcile() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    record(&mut app, account, "1000", TransactionKind::Income);
    record(&mut app, account, "100", TransactionKind::Expense);
    let rent = add(&mut app, account, "ค่าเช่า", "200", "1", "2026-09");
    add(&mut app, account, "ค่าสมาชิก", "300", "31", "2026-09");
    let before = projection(&mut app);
    assert_eq!(before.expenses.to_string(), "600.00");
    assert_eq!(before.rate(), Some(2500));
    let balance = view(&mut app).accounts[0].balance;
    let prepared = payment(&mut app, &rent, month(), "200");
    assert_eq!(
        view(&mut app).accounts[0].balance,
        balance,
        "preview cannot spend"
    );
    app.execute(Command::PayRecurring(prepared.clone()))
        .expect("pay");
    app.execute(Command::PayRecurring(prepared.clone()))
        .expect("retry");
    let v = view(&mut app);
    assert_eq!(v.expenses.to_string(), "300.00");
    assert_eq!(
        v.accounts[0].balance,
        balance
            .checked_sub("200".parse().expect("money"))
            .expect("balance")
    );
    let totals = recurring_month(&v, month()).expect("totals");
    assert_eq!(totals.planned.to_string(), "500.00");
    assert_eq!(totals.paid.to_string(), "200.00");
    assert_eq!(totals.pending.to_string(), "300.00");
    assert_eq!(projection(&mut app), before);
    let mut duplicate = prepared.clone();
    duplicate.payment.submission = RandomIds.submission_id().expect("id");
    assert!(app.execute(Command::PayRecurring(duplicate)).is_err());
    app.execute(Command::Reverse(prepared.payment.entry.id()))
        .expect("reverse");
    assert_eq!(projection(&mut app), before);
    assert_eq!(
        recurring_month(&view(&mut app), month())
            .expect("totals")
            .pending
            .to_string(),
        "500.00"
    );
    let replacement = payment(&mut app, &rent, month(), "250");
    app.execute(Command::PayRecurring(replacement))
        .expect("new payment");
    assert_eq!(
        projection(&mut app).expenses.to_string(),
        "650.00",
        "actual payment can differ from plan"
    );
}

#[test]
fn existing_expense_can_settle_only_one_obligation_and_does_not_add_a_journal() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let one = add(&mut app, account, "ค่าเช่า", "200", "1", "2026-09");
    let two = add(&mut app, account, "ค่าไฟ", "200", "2", "2026-09");
    let recorded = record(&mut app, account, "200", TransactionKind::Expense);
    let before = view(&mut app).entries.len();
    let command = Command::LinkRecurring {
        expected: one.clone(),
        month: month(),
        entry: recorded.entry.id(),
    };
    app.execute(command.clone()).expect("link");
    app.execute(command).expect("retry link");
    assert_eq!(view(&mut app).entries.len(), before);
    assert_eq!(projection(&mut app).expenses.to_string(), "400.00");
    assert!(
        app.execute(Command::LinkRecurring {
            expected: two,
            month: month(),
            entry: recorded.entry.id()
        })
        .is_err()
    );
    let income = record(&mut app, account, "1000", TransactionKind::Income);
    assert!(
        app.execute(Command::LinkRecurring {
            expected: one,
            month: month().shifted(1).expect("next"),
            entry: income.entry.id()
        })
        .is_err()
    );
}

#[test]
fn prepayment_settles_future_period_without_moving_its_cashflow_date() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let next = month().shifted(1).expect("next");
    let s = add(&mut app, account, "ค่าเช่า", "200", "31", "2026-10");
    let prepared = payment(&mut app, &s, next, "200");
    app.execute(Command::PayRecurring(prepared))
        .expect("pay early");
    assert_eq!(projection(&mut app).expenses.to_string(), "200.00");
    let v = view(&mut app);
    assert_eq!(
        recurring_month(&v, next).expect("totals").pending,
        Money::ZERO
    );
    assert_eq!(
        projected_cashflow(
            &v,
            &MonthlyFlow {
                year: 2026,
                month: 10,
                income: Money::ZERO,
                expenses: Money::ZERO
            }
        )
        .expect("forecast")
        .expenses,
        Money::ZERO
    );
}

#[test]
fn payment_and_link_rollback_when_settlement_write_fails() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("test.db");
    let (mut app, account) = setup(SqliteLedger::open(&path).expect("db"));
    let s = add(&mut app, account, "ค่าเช่า", "200", "1", "2026-09");
    let p = payment(&mut app, &s, month(), "200");
    let before = view(&mut app);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER reject_settlement BEFORE INSERT ON recurring_settlements BEGIN SELECT RAISE(ABORT, 'test'); END;").expect("trigger");
    assert!(app.execute(Command::PayRecurring(p.clone())).is_err());
    assert_eq!(
        view(&mut app),
        before,
        "journals and postings must roll back too"
    );
    raw.execute_batch("DROP TRIGGER reject_settlement")
        .expect("drop");
    app.execute(Command::PayRecurring(p)).expect("retry");
}

#[test]
fn stale_payment_is_rejected_after_another_connection_stops_the_plan() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("test.db");
    let (mut first, account) = setup(SqliteLedger::open(&path).expect("db"));
    let s = add(&mut first, account, "ค่าเช่า", "200", "1", "2026-09");
    let prepared = payment(&mut first, &s, month(), "200");
    let mut second = app(SqliteLedger::open(&path).expect("second"));
    second
        .execute(Command::StopRecurring {
            expected: s.clone(),
            month: month(),
        })
        .expect("stop");
    assert!(first.execute(Command::PayRecurring(prepared)).is_err());
    assert_eq!(view(&mut first).expenses, Money::ZERO);
    assert!(
        recurring_month(&view(&mut first), month())
            .expect("totals")
            .items
            .is_empty()
    );
    assert!(
        first
            .execute(Command::StopRecurring {
                expected: s,
                month: month()
            })
            .is_err()
    );
}

#[test]
fn deleting_payment_account_keeps_plan_and_requires_a_new_account() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let s = add(&mut app, account, "ค่าเช่า", "200", "1", "2026-09");
    let p = payment(&mut app, &s, month(), "200");
    app.execute(Command::PayRecurring(p)).expect("pay");
    let Response::AccountDeletion(deletion) = app
        .execute(Command::PreviewDeleteAccount(account))
        .expect("preview")
    else {
        panic!("deletion")
    };
    app.execute(Command::DeleteAccount(deletion))
        .expect("delete");
    let v = view(&mut app);
    assert_eq!(v.recurring.len(), 1);
    assert_eq!(v.recurring[0].account(), None);
    assert!(v.settlements.is_empty());
    assert_eq!(
        recurring_month(&v, month())
            .expect("totals")
            .pending
            .to_string(),
        "200.00"
    );
}

#[test]
fn v1_database_migrates_without_changing_old_journals_and_reopens_plans() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("test.db");
    let (mut initial, account) = setup(SqliteLedger::open(&path).expect("db"));
    record(&mut initial, account, "99.99", TransactionKind::Expense);
    let before = view(&mut initial);
    drop(initial);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch(
        "DROP TABLE crypto_holding_changes; DROP TABLE crypto_prices; ALTER TABLE accounts DROP COLUMN sol_atoms; ALTER TABLE accounts DROP COLUMN btc_atoms; ALTER TABLE accounts DROP COLUMN payment_day; ALTER TABLE accounts DROP COLUMN closing_day; DROP TABLE user_preferences; DROP TRIGGER lock_currency_accounts; DROP TRIGGER lock_currency_recurring; DROP TRIGGER lock_currency_receivables; DROP TABLE ledger_settings; DROP TABLE prompt_submissions; DROP TABLE receivables; DROP TABLE recurring_settlements; DROP TABLE recurring_expenses; PRAGMA user_version=1;",
    )
    .expect("v1 fixture");
    let mut migrated = app(SqliteLedger::open(&path).expect("migration"));
    assert_eq!(view(&mut migrated), before);
    let s = add(&mut migrated, account, "ค่าเช่า", "200", "31", "2026-09");
    let p = payment(&mut migrated, &s, month(), "200");
    migrated.execute(Command::PayRecurring(p)).expect("pay");
    let saved = view(&mut migrated);
    drop(migrated);
    assert_eq!(
        view(&mut app(SqliteLedger::open(&path).expect("reopen"))),
        saved
    );
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
        .expect("integrity"),
        0
    );
}

#[test]
fn invalid_schedule_fields_and_aggregate_overflow_are_rejected() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    for (amount, day, category) in [
        ("0", "1", Category::Rent),
        ("1", "0", Category::Rent),
        ("1", "32", Category::Rent),
        ("1", "1", Category::Salary),
    ] {
        assert!(
            app.execute(Command::AddRecurring(RecurringInput {
                installments: None,
                name: "ค่าเช่า".into(),
                amount: amount.into(),
                day: day.into(),
                start: "2026-09".into(),
                account: Some(account),
                category: Some(category)
            }))
            .is_err()
        );
    }
    add(&mut app, account, "มากสุด", "90000000000", "1", "2026-09");
    assert!(
        app.execute(Command::AddRecurring(RecurringInput {
            installments: None,
            name: "เพิ่ม".into(),
            amount: "1".into(),
            day: "1".into(),
            start: "2026-10".into(),
            account: Some(account),
            category: Some(Category::Rent)
        }))
        .is_err()
    );
}

fn finite(
    app: &mut App,
    account: AccountId,
    name: &str,
    start: &str,
    count: &str,
) -> RecurringExpense {
    app.execute(Command::AddRecurring(RecurringInput {
        name: name.into(),
        amount: "200".into(),
        day: "31".into(),
        start: start.into(),
        account: Some(account),
        category: Some(Category::Rent),
        installments: Some(count.into()),
    }))
    .expect("finite plan");
    view(app)
        .recurring
        .into_iter()
        .find(|s| s.name().as_str() == name)
        .expect("created")
}
fn selected_preview(app: &mut App, schedule: &RecurringExpense, month: Month) -> PreparedEntry {
    let input = select_recurring(
        &EntryInput::empty("2026-09-24".parse().expect("date")),
        schedule,
        month,
    );
    match app
        .execute(Command::Preview(input))
        .expect("preview selection")
    {
        Response::Prepared(p) => p,
        _ => panic!("prepared"),
    }
}

#[test]
fn finite_plan_finishes_only_after_all_periods_paid_and_reversal_reopens_original() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let plan = finite(&mut app, account, "ผ่อนโทรศัพท์", "2026-07", "3");
    let july: Month = "2026-07".parse().expect("july");
    assert!(!plan.occurs_in("2026-10".parse().expect("october")));
    assert_eq!(plan.due().number_in(month()), Some(3));
    // A later installment paid first must not hide the earlier debt.
    let last = selected_preview(&mut app, &plan, month());
    app.execute(Command::Commit(last)).expect("ordinary save");
    let p = recurring_progress(&view(&mut app)).remove(0);
    assert_eq!(
        (p.paid, p.remaining, p.next_unpaid),
        (1, Some(2), Some(july))
    );
    let first = selected_preview(&mut app, &plan, july);
    let second = selected_preview(&mut app, &plan, july.shifted(1).expect("august"));
    app.execute(Command::CommitBatch(vec![first.clone(), second.clone()]))
        .expect("settle batch");
    app.execute(Command::CommitBatch(vec![first, second.clone()]))
        .expect("idempotent batch");
    let v = view(&mut app);
    let p = &recurring_progress(&v)[0];
    assert!(p.completed);
    assert_eq!(p.remaining, Some(0));
    assert_eq!(p.next_unpaid, None);
    assert_eq!(v.expenses.to_string(), "600.00");
    assert_eq!(
        recurring_month(&v, "2026-10".parse().expect("october"))
            .expect("month")
            .pending,
        Money::ZERO
    );
    let future = projected_cashflow(
        &v,
        &MonthlyFlow {
            year: 2026,
            month: 10,
            income: Money::ZERO,
            expenses: Money::ZERO,
        },
    )
    .expect("flow");
    assert_eq!(future.rate(), None);
    app.execute(Command::Reverse(second.entry.id()))
        .expect("reverse");
    let p = recurring_progress(&view(&mut app)).remove(0);
    assert!(!p.completed);
    assert_eq!(p.remaining, Some(1));
    assert_eq!(p.next_unpaid, Some(july.shifted(1).expect("august")));
    let replacement = selected_preview(&mut app, &plan, p.next_unpaid.expect("unpaid"));
    app.execute(Command::Commit(replacement))
        .expect("replace cancelled installment");
    assert!(recurring_progress(&view(&mut app))[0].completed);
}

#[test]
fn prompt_batch_can_select_a_plan_without_overwriting_explicit_prompt_fields() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let plan = finite(&mut app, account, "ค่าใช้จ่ายผ่อน", "2026-09", "2");
    let Response::Resolved(QuickResolution::Batch { drafts, .. }) = app.execute(Command::Resolve("เมื่อวานซื้อของ 190 บาท บันทึกลงเงินสด หมวด ของใช้ แล้วก็วันนี้ได้เงินค่าจ้าง 400 บาท บันทึกลงเงินสด หมวด ฟรีแลนซ์".into())).expect("resolve") else { panic!("batch") };
    let mut inputs: Vec<_> = drafts.into_iter().map(|d| d.input).collect();
    let original = inputs[0].clone();
    inputs[0] = select_recurring(&inputs[0], &plan, month());
    assert_eq!(inputs[0].amount, original.amount);
    assert_eq!(inputs[0].date, original.date);
    assert_eq!(inputs[0].category, original.category);
    assert_eq!(inputs[0].account, original.account);
    let Response::PreparedBatch(prepared) = app
        .execute(Command::PreviewBatch(inputs))
        .expect("preview batch")
    else {
        panic!("preview")
    };
    app.execute(Command::CommitBatch(prepared.clone()))
        .expect("commit batch");
    app.execute(Command::CommitBatch(prepared)).expect("retry");
    let v = view(&mut app);
    assert_eq!(v.income.to_string(), "400.00");
    assert_eq!(v.expenses.to_string(), "190.00");
    assert_eq!(v.settlements.len(), 1);
    assert_eq!(recurring_progress(&v)[0].remaining, Some(1));
    assert_eq!(
        recurring_month(&v, month()).expect("totals").pending,
        Money::ZERO
    );
    let mut invalid = select_recurring(&EntryInput::empty(v.today), &plan, month());
    invalid.recurring.as_mut().expect("selected").month = "bad month".into();
    assert!(missing_entry_fields(&invalid).contains(&"งวดเดือนที่ต้องการจ่าย (YYYY-MM)"));
    assert!(app.execute(Command::Preview(invalid)).is_err());
}

#[test]
fn duplicate_installment_in_one_batch_is_rejected_before_and_during_commit() {
    let (mut app, account) = setup(SqliteLedger::in_memory().expect("db"));
    let plan = finite(&mut app, account, "ผ่อน", "2026-09", "2");
    let input = select_recurring(
        &EntryInput::empty("2026-09-24".parse().expect("date")),
        &plan,
        month(),
    );
    assert!(
        app.execute(Command::PreviewBatch(vec![input.clone(), input]))
            .is_err()
    );
    let one = selected_preview(&mut app, &plan, month());
    let two = selected_preview(&mut app, &plan, month());
    let before = view(&mut app);
    assert!(app.execute(Command::CommitBatch(vec![one, two])).is_err());
    assert_eq!(
        view(&mut app),
        before,
        "first journal and settlement must roll back"
    );
}

#[test]
fn failure_on_second_selected_installment_rolls_back_both_journals_and_links() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("test.db");
    let (mut app, account) = setup(SqliteLedger::open(&path).expect("db"));
    let plan = finite(&mut app, account, "ผ่อน", "2026-09", "2");
    let one = selected_preview(&mut app, &plan, month());
    let two = selected_preview(&mut app, &plan, month().shifted(1).expect("october"));
    let before = view(&mut app);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch("CREATE TRIGGER reject_october BEFORE INSERT ON recurring_settlements WHEN NEW.month='2026-10' BEGIN SELECT RAISE(ABORT, 'test'); END;").expect("trigger");
    assert!(
        app.execute(Command::CommitBatch(vec![one.clone(), two.clone()]))
            .is_err()
    );
    assert_eq!(view(&mut app), before);
    raw.execute_batch("DROP TRIGGER reject_october")
        .expect("drop");
    app.execute(Command::CommitBatch(vec![one, two]))
        .expect("retry");
    assert!(recurring_progress(&view(&mut app))[0].completed);
}

#[test]
fn count_edits_validate_history_stale_previews_and_unlimited_continuation() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("test.db");
    let (mut first, account) = setup(SqliteLedger::open(&path).expect("db"));
    let plan = finite(&mut first, account, "ผ่อน", "2026-09", "3");
    let stale = selected_preview(&mut first, &plan, month().shifted(2).expect("november"));
    let mut second = app(SqliteLedger::open(&path).expect("second"));
    second
        .execute(Command::SetRecurringInstallments {
            expected: plan.clone(),
            installments: Some("2".into()),
        })
        .expect("shorter plan");
    assert!(first.execute(Command::Commit(stale)).is_err());
    let plan = view(&mut first).recurring[0].clone();
    let last = selected_preview(&mut first, &plan, month().shifted(1).expect("october"));
    first
        .execute(Command::Commit(last))
        .expect("pay final early");
    assert!(
        first
            .execute(Command::SetRecurringInstallments {
                expected: plan.clone(),
                installments: Some("1".into())
            })
            .is_err()
    );
    first
        .execute(Command::SetRecurringInstallments {
            expected: plan,
            installments: None,
        })
        .expect("unlimited");
    let v = view(&mut first);
    let p = &recurring_progress(&v)[0];
    assert_eq!(p.remaining, None);
    assert!(!p.completed);
    assert_eq!(p.next_unpaid, Some(month()));
    assert!(v.recurring[0].occurs_in("2030-01".parse().expect("future")));
}

#[test]
fn count_limits_and_calendar_end_are_validated() {
    for count in ["", "0", "-1", "1.5", "1201", "9999999999999"] {
        assert!(parse_installments(Some(count)).is_err(), "{count}");
    }
    assert_eq!(parse_installments(None).expect("unlimited"), None);
    assert_eq!(parse_installments(Some("1200")).expect("max"), Some(1200));
    assert!(
        MonthlyDue::new(31, "9999-12".parse().expect("month"))
            .expect("due")
            .with_installments(Some(2))
            .is_err()
    );
    let due = MonthlyDue::new(31, "2023-12".parse().expect("month"))
        .expect("due")
        .with_installments(Some(3))
        .expect("three");
    assert_eq!(due.number_in("2024-02".parse().expect("leap")), Some(3));
    assert_eq!(due.number_in("2024-03".parse().expect("after")), None);
}

#[test]
fn populated_v2_migrates_as_unlimited_and_new_count_survives_reopen() {
    let dir = TempDir::new().expect("dir");
    let path = dir.path().join("test.db");
    let (mut first, account) = setup(SqliteLedger::open(&path).expect("db"));
    let plan = add(&mut first, account, "แผนเดิม", "200", "31", "2026-09");
    let paid = payment(&mut first, &plan, month(), "200");
    first.execute(Command::PayRecurring(paid)).expect("paid");
    let before = view(&mut first);
    drop(first);
    let raw = Connection::open(&path).expect("raw");
    raw.execute_batch(
        "DROP TABLE crypto_holding_changes; DROP TABLE crypto_prices; ALTER TABLE accounts DROP COLUMN sol_atoms; ALTER TABLE accounts DROP COLUMN btc_atoms; ALTER TABLE accounts DROP COLUMN payment_day; ALTER TABLE accounts DROP COLUMN closing_day; DROP TABLE user_preferences; DROP TRIGGER lock_currency_accounts; DROP TRIGGER lock_currency_recurring; DROP TRIGGER lock_currency_receivables; DROP TABLE ledger_settings; DROP TABLE prompt_submissions; DROP TABLE receivables; ALTER TABLE recurring_expenses DROP COLUMN installments; PRAGMA user_version=2;",
    )
    .expect("populated v2 fixture");
    let mut migrated = app(SqliteLedger::open(&path).expect("migrate"));
    assert_eq!(view(&mut migrated), before);
    migrated
        .execute(Command::SetRecurringInstallments {
            expected: plan,
            installments: Some("1".into()),
        })
        .expect("one installment");
    drop(migrated);
    let v = view(&mut app(SqliteLedger::open(&path).expect("reopen")));
    assert!(recurring_progress(&v)[0].completed);
    assert_eq!(v.expenses.to_string(), "200.00");
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .expect("schema"),
        10
    );
}
