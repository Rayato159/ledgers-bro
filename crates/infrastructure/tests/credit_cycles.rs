#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{RandomIds, SqliteLedger};

struct Today;
impl Clock for Today {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok("2026-09-25".parse()?)
    }
}
type App = LedgerApplication<SqliteLedger, Today, RandomIds>;
fn view(app: &mut App) -> Dashboard {
    let Response::Dashboard(v) = app.execute(Command::Load).expect("load") else {
        panic!("dashboard")
    };
    v
}
fn account(app: &mut App, name: &str, kind: AccountKind, opening: &str) -> AccountId {
    app.execute(Command::CreateAccount {
        name: name.into(),
        kind,
        opening: opening.into(),
        credit_cycle: if kind == AccountKind::CreditCard {
            Some(CreditCardCycle::new(20, 5).expect("cycle"))
        } else {
            None
        },
    })
    .expect("create");
    view(app)
        .accounts
        .iter()
        .find(|a| a.account.name().as_str() == name)
        .expect("account")
        .account
        .id()
}
fn commit(app: &mut App, input: EntryInput) -> EntryId {
    let Response::Prepared(p) = app.execute(Command::Preview(input)).expect("preview") else {
        panic!("prepared")
    };
    let id = p.entry.id();
    app.execute(Command::Commit(p)).expect("commit");
    id
}
fn expense(app: &mut App, from: AccountId, amount: &str, date: &str) -> EntryId {
    commit(
        app,
        EntryInput {
            account: Some(from),
            amount: amount.into(),
            category: Some(Category::Food),
            date: date.into(),
            ..EntryInput::empty("2026-09-25".parse().expect("date"))
        },
    )
}
fn pay(app: &mut App, from: AccountId, to: AccountId, amount: &str) -> EntryId {
    commit(
        app,
        EntryInput {
            kind: TransactionKind::Transfer,
            account: Some(from),
            destination: Some(to),
            amount: amount.into(),
            ..EntryInput::empty("2026-09-25".parse().expect("date"))
        },
    )
}

#[test]
fn purchases_partial_payments_overpayments_and_reversals_reconcile_exactly() {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let bank = account(&mut app, "bank", AccountKind::Bank, "1000");
    let card = account(&mut app, "card", AccountKind::CreditCard, "0");
    expense(&mut app, card, "100", "2026-09-20");
    let purchase = expense(&mut app, card, "200", "2026-09-21");
    expense(&mut app, bank, "50", "2026-09-22");
    pay(&mut app, bank, card, "50");
    let v = view(&mut app);
    let cards = credit_cards(&v).expect("bills");
    assert_eq!(
        cards[0].bills[0].dates.expect("dates").due.to_string(),
        "2026-10-05"
    );
    assert_eq!(cards[0].bills[0].outstanding.to_string(), "50.00");
    assert_eq!(
        cards[0].bills[1].dates.expect("dates").due.to_string(),
        "2026-11-05"
    );
    assert_eq!(cards[0].outstanding.to_string(), "250.00");
    assert_eq!(v.assets.to_string(), "900.00");
    assert_eq!(v.net_worth.to_string(), "650.00");
    assert_eq!(v.expenses.to_string(), "350.00");
    assert_eq!(paid_out_this_month(&v).expect("paid").to_string(), "100.00");
    let payment = pay(&mut app, bank, card, "300");
    let v = view(&mut app);
    let cards = credit_cards(&v).expect("bills");
    assert_eq!(cards[0].outstanding, Money::ZERO);
    assert_eq!(cards[0].prepaid.to_string(), "50.00");
    assert_eq!(v.net_worth.to_string(), "650.00");
    assert_eq!(
        v.expenses.to_string(),
        "350.00",
        "settlement is not a second expense"
    );
    app.execute(Command::Reverse(payment))
        .expect("reverse payment");
    app.execute(Command::Reverse(purchase))
        .expect("reverse purchase");
    let v = view(&mut app);
    assert_eq!(
        credit_cards(&v).expect("bills")[0].outstanding.to_string(),
        "50.00"
    );
    assert_eq!(v.liabilities.to_string(), "50.00");
    assert_eq!(v.net_worth.to_string(), "850.00");
    assert_eq!(paid_out_this_month(&v).expect("paid").to_string(), "100.00");
}

#[test]
fn pending_recurring_plan_does_not_duplicate_an_expense_charged_to_card() {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let card = account(&mut app, "card", AccountKind::CreditCard, "0");
    app.execute(Command::AddRecurring(RecurringInput {
        name: "Internet".into(),
        amount: "500".into(),
        day: "10".into(),
        start: "2026-09".into(),
        account: Some(card),
        category: Some(Category::Supplies),
        installments: None,
    }))
    .expect("plan");
    let before = view(&mut app);
    let schedule = before.recurring[0].clone();
    let month = "2026-09".parse().expect("month");
    assert_eq!(
        recurring_month(&before, month)
            .expect("plan")
            .pending
            .to_string(),
        "500.00"
    );
    let input = EntryInput {
        account: Some(card),
        amount: "500".into(),
        category: Some(Category::Supplies),
        date: "2026-09-10".into(),
        ..EntryInput::empty(before.today)
    };
    let Response::PreparedRecurringPayment(p) = app
        .execute(Command::PreviewRecurringPayment {
            expected: schedule,
            month,
            input,
        })
        .expect("preview")
    else {
        panic!("prepared")
    };
    app.execute(Command::PayRecurring(p)).expect("pay");
    let after = view(&mut app);
    assert_eq!(
        recurring_month(&after, month).expect("plan").pending,
        Money::ZERO
    );
    assert_eq!(
        credit_cards(&after).expect("card")[0]
            .outstanding
            .to_string(),
        "500.00"
    );
    assert_eq!(paid_out_this_month(&after).expect("cash"), Money::ZERO);
    assert_eq!(after.expenses.to_string(), "500.00");
}

#[test]
fn missing_terms_are_rejected_and_terms_survive_reopening() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    assert!(
        app.execute(Command::CreateAccount {
            name: "card".into(),
            kind: AccountKind::CreditCard,
            opening: "0".into(),
            credit_cycle: None
        })
        .is_err()
    );
    assert!(view(&mut app).accounts.is_empty());
    account(&mut app, "card", AccountKind::CreditCard, "80");
    drop(app);
    let mut app = App::new(SqliteLedger::open(&path).expect("reopen"), Today, RandomIds);
    let v = view(&mut app);
    assert_eq!(
        v.accounts[0].account.credit_cycle(),
        Some(CreditCardCycle::new(20, 5).expect("cycle"))
    );
    let cards = credit_cards(&v).expect("bills");
    assert_eq!(
        cards[0].bills[0].dates, None,
        "do not invent a due date for a carried balance"
    );
    assert_eq!(cards[0].outstanding.to_string(), "80.00");
    assert_eq!(cards[0].overdue, Money::ZERO);
}

#[test]
fn version_seven_cards_can_set_terms_once_without_losing_accounts() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let connection = rusqlite::Connection::open(&path).expect("db");
    for schema in [
        include_str!("../migrations/001_ledger.sql"),
        include_str!("../migrations/002_recurring.sql"),
        include_str!("../migrations/003_installments.sql"),
        include_str!("../migrations/004_receivables.sql"),
        include_str!("../migrations/005_prompt_submissions.sql"),
        include_str!("../migrations/006_currency.sql"),
        include_str!("../migrations/007_preferences.sql"),
    ] {
        connection.execute_batch(schema).expect("v7 schema");
    }
    connection.execute("INSERT INTO accounts(id,name,name_key,kind,archived) VALUES(?1,'old card','old card','credit',0)",["00000000-0000-0000-0000-000000000001"]).expect("legacy account");
    drop(connection);
    let mut repo = SqliteLedger::open(&path).expect("migration");
    let old = repo.snapshot().expect("snapshot").accounts[0].clone();
    assert_eq!(old.credit_cycle(), None);
    let cycle = CreditCardCycle::new(20, 5).expect("cycle");
    repo.set_credit_cycle(&old, cycle).expect("set terms");
    assert_eq!(
        repo.set_credit_cycle(&old, cycle),
        Err(StorageError::CreditCycleChanged)
    );
    drop(repo);
    let mut repo = SqliteLedger::open(&path).expect("reopen");
    assert_eq!(
        repo.snapshot().expect("snapshot").accounts[0].credit_cycle(),
        Some(cycle)
    );
    drop(repo);
    let connection = rusqlite::Connection::open(&path).expect("db");
    assert!(
        connection
            .execute("UPDATE accounts SET payment_day=NULL", [])
            .is_err()
    );
    assert!(
        connection
            .execute("UPDATE accounts SET closing_day=NULL", [])
            .is_err()
    );
    assert!(
        connection
            .execute("UPDATE accounts SET payment_day=32", [])
            .is_err()
    );
}
