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
    let Response::Dashboard(view) = app.execute(Command::Load).expect("load") else {
        panic!("dashboard")
    };
    view
}
fn account(app: &mut App) -> AccountId {
    app.execute(Command::CreateAccount {
        name: "Bank".into(),
        kind: AccountKind::Bank,
        opening: "1000".into(),
        credit_cycle: None,
    })
    .expect("account");
    view(app).accounts[0].account.id()
}
fn income(bank: AccountId, section: u8, date: &str) -> EntryInput {
    EntryInput {
        kind: TransactionKind::Income,
        amount: "10400".into(),
        account: Some(bank),
        category: Some(Category::Freelance),
        date: date.into(),
        income_tax: Some(Box::new(IncomeTaxInput {
            section: Some(IncomeSection::new(section).expect("section")),
            gross: "10000".into(),
            withholding: "300".into(),
            vat: "700".into(),
            other_deductions: "0".into(),
        })),
        ..EntryInput::empty("2026-09-25".parse().expect("date"))
    }
}
fn save(app: &mut App, input: EntryInput) -> EntryId {
    let Response::Prepared(prepared) = app.execute(Command::Preview(input)).expect("preview")
    else {
        panic!("preview")
    };
    app.execute(Command::Commit(prepared.clone()))
        .expect("save");
    app.execute(Command::Commit(prepared.clone()))
        .expect("retry is idempotent");
    prepared.entry.id()
}

#[test]
fn annual_income_keeps_vat_and_withholding_separate_and_recomputes_after_reversal() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    let bank = account(&mut app);
    let mut ids = Vec::new();
    for section in 1..=8 {
        ids.push(save(&mut app, income(bank, section, "2026-09-01")));
    }
    save(&mut app, income(bank, 2, "2025-12-31"));
    let mut unselected = income(bank, 2, "2026-09-01");
    unselected.income_tax = None;
    save(&mut app, unselected);
    let before = view(&mut app);
    drop(app);
    let mut app = App::new(SqliteLedger::open(&path).expect("reopen"), Today, RandomIds);
    assert_eq!(view(&mut app), before, "all evidence persists exactly");
    let totals = annual_tax_income(&before, 2569).expect("year");
    assert_eq!(totals.count, 8);
    assert!(
        totals
            .incomes
            .iter()
            .all(|money| money.to_string() == "10000.00")
    );
    assert_eq!(totals.withholding.to_string(), "2400.00");
    assert_eq!(annual_tax_income(&before, 2568).expect("previous").count, 1);
    let manual = TaxWorksheet {
        withholding: "10".into(),
        eligibility_confirmed: true,
        ..Default::default()
    };
    let combined = tax_worksheet_with_entries(&manual, &totals).expect("merge");
    assert_eq!(combined.withholding, "2410.00");
    assert_eq!(
        tax_worksheet_with_entries(&manual, &totals).expect("again"),
        combined
    );
    assert_eq!(manual.incomes[0], "0", "no feedback into manual fields");
    assert_eq!(
        calculate_tax(&combined)
            .expect("calculate")
            .gross
            .to_string(),
        "80000.00"
    );
    app.execute(Command::Reverse(ids[0])).expect("reverse");
    let changed = annual_tax_income(&view(&mut app), 2569).expect("changed");
    assert_eq!(changed.count, 7);
    assert_eq!(changed.incomes[0], Money::ZERO);
    assert_eq!(changed.withholding.to_string(), "2100.00");
}

#[test]
fn tax_selection_requires_type_and_exact_net_receipt_and_is_never_attached_to_expenses() {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let bank = account(&mut app);
    let base = income(bank, 1, "2026-09-01");
    for mutation in 0..5 {
        let mut input = base.clone();
        let tax = input.income_tax.as_mut().expect("tax");
        match mutation {
            0 => tax.section = None,
            1 => tax.withholding = "-1".into(),
            2 => input.amount = "10000".into(),
            3 => {
                input.kind = TransactionKind::Expense;
                input.category = Some(Category::Food);
            }
            _ => tax.gross = "0".into(),
        }
        assert!(app.execute(Command::Preview(input)).is_err());
    }
    let mut salary = base;
    salary.amount = "28750".into();
    salary.income_tax = Some(Box::new(IncomeTaxInput {
        gross: "30000".into(),
        withholding: "500".into(),
        other_deductions: "750".into(),
        vat: "0".into(),
        section: Some(IncomeSection::new(1).expect("salary")),
    }));
    save(&mut app, salary);
    let totals = annual_tax_income(&view(&mut app), 2569).expect("totals");
    assert_eq!(totals.incomes[0].to_string(), "30000.00");
    assert_eq!(totals.withholding.to_string(), "500.00");
}

#[test]
fn upgrading_schema_eight_preserves_accounts_transactions_cycles_and_preferences() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    let bank = account(&mut app);
    app.execute(Command::CreateAccount {
        name: "Old card".into(),
        kind: AccountKind::CreditCard,
        opening: "419.00".into(),
        credit_cycle: Some(CreditCardCycle::new(20, 10).expect("cycle")),
    })
    .expect("card");
    let mut old = income(bank, 1, "2026-09-01");
    old.income_tax = None;
    save(&mut app, old);
    app.execute(Command::AddRecurring(RecurringInput {
        name: "Rent".into(),
        amount: "2000".into(),
        day: "5".into(),
        start: "2026-09".into(),
        account: Some(bank),
        category: Some(Category::Rent),
        installments: None,
    }))
    .expect("plan");
    let preferences = UserPreferences {
        dark: true,
        english: true,
        ..Default::default()
    };
    app.execute(Command::SetPreferences(preferences))
        .expect("preferences");
    let before = view(&mut app);
    drop(app);
    // V9 only adds optional JSON evidence and a reader-version gate. Unannotated
    // entries serialize exactly as v8; use that previous on-disk format here.
    let db = rusqlite::Connection::open(&path).expect("old file");
    db.pragma_update(None, "user_version", 8).expect("v8");
    drop(db);
    let mut app = App::new(
        SqliteLedger::open(&path).expect("upgrade"),
        Today,
        RandomIds,
    );
    assert_eq!(view(&mut app), before);
    let Response::Preferences(saved) = app.execute(Command::LoadPreferences).expect("prefs") else {
        panic!("prefs")
    };
    assert_eq!(saved, preferences);
    assert_eq!(
        annual_tax_income(&view(&mut app), 2569).expect("unmarked"),
        AnnualTaxIncome::default()
    );
    save(&mut app, income(bank, 2, "2026-09-25"));
    drop(app);
    let db = rusqlite::Connection::open(&path).expect("read upgraded");
    assert_eq!(
        db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .expect("version"),
        9
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |row| row.get::<_, String>(0))
            .expect("integrity"),
        "ok"
    );
}
