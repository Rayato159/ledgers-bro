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
const NOW: i64 = 1_790_330_700;
fn view(app: &mut App) -> Dashboard {
    let Response::Dashboard(v) = app.execute(Command::Load).expect("load") else {
        panic!("dashboard")
    };
    v
}
fn prices(time: i64, btc: &str, sol: &str) -> CryptoPrices {
    CryptoPrices {
        bitcoin: CryptoQuote::new(ThbUnitPrice::parse(btc).expect("price"), time).expect("quote"),
        solana: CryptoQuote::new(ThbUnitPrice::parse(sol).expect("price"), time).expect("quote"),
    }
}
fn holdings(btc: &str, sol: &str) -> CryptoHoldings {
    CryptoHoldings::parse(btc, sol).expect("holdings")
}
fn create(app: &mut App, name: &str, btc: &str, sol: &str) -> Account {
    app.execute(Command::CreateCryptoAccount {
        name: name.into(),
        holdings: holdings(btc, sol),
    })
    .expect("create");
    view(app)
        .accounts
        .into_iter()
        .find(|a| a.account.name().as_str() == name)
        .expect("account")
        .account
}
fn legacy(app: &mut App) -> Account {
    app.execute(Command::CreateAccount {
        name: "Old crypto".into(),
        kind: AccountKind::Crypto,
        opening: "10000".into(),
        credit_cycle: None,
    })
    .expect("legacy");
    view(app).accounts[0].account.clone()
}

#[test]
fn multi_coin_portfolio_persists_exact_units_and_cache_without_posting_revaluations() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    let original = create(&mut app, "BTC + SOL", "0.12345678", "2.123456789");
    let before = view(&mut app);
    let quote = prices(NOW, "2815675.95613859", "3937.37225198");
    app.execute(Command::SaveCryptoPrices(quote))
        .expect("save prices");
    app.execute(Command::SaveCryptoPrices(prices(NOW - 60, "1", "1")))
        .expect("old quote ignored");
    app.execute(Command::SetCryptoHoldings {
        expected: original.clone(),
        holdings: holdings("0.00000001", "0.000000001"),
    })
    .expect("edit");
    assert!(
        app.execute(Command::SetCryptoHoldings {
            expected: original,
            holdings: holdings("99", "99")
        })
        .is_err()
    );
    let after = view(&mut app);
    assert_eq!(before.entries, after.entries);
    assert_eq!(after.income, Money::ZERO);
    assert_eq!(after.expenses, Money::ZERO);
    drop(app);
    let mut app = App::new(SqliteLedger::open(&path).expect("reopen"), Today, RandomIds);
    assert_eq!(view(&mut app), after);
    let Response::CryptoPrices(saved) = app.execute(Command::LoadCryptoPrices).expect("cache")
    else {
        panic!("cache")
    };
    assert_eq!(saved, Some(quote));
    let db = rusqlite::Connection::open(path).expect("inspect");
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM crypto_holding_changes", [], |r| r
            .get::<_, i64>(0))
            .expect("audit"),
        2
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .expect("integrity"),
        "ok"
    );
}

#[test]
fn legacy_conversion_replaces_book_value_and_distinguishes_unpriced_from_zero() {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let account = legacy(&mut app);
    let before = view(&mut app);
    assert_eq!(
        crypto_valuation(&before, None, NOW)
            .expect("legacy")
            .assets
            .to_string(),
        "10000.00"
    );
    app.execute(Command::SetCryptoHoldings {
        expected: account,
        holdings: holdings("0.01", "2"),
    })
    .expect("convert");
    let after = view(&mut app);
    assert_eq!(before.entries, after.entries);
    assert_eq!(
        after.accounts[0].balance.to_string(),
        "10000.00",
        "old ledger retained"
    );
    let live =
        crypto_valuation(&after, Some(prices(NOW, "3000000", "5000")), NOW).expect("valuation");
    assert_eq!(
        live.assets.to_string(),
        "40000.00",
        "not 50000: no double counting"
    );
    assert_eq!(live.net_worth, live.assets);
    let unknown = crypto_valuation(&after, None, NOW).expect("unpriced");
    assert_eq!(unknown.unpriced_portfolios, 1);
    assert_eq!(unknown.assets, Money::ZERO);
    assert_eq!(
        crypto_valuation(&after, Some(prices(NOW + 301, "1", "1")), NOW)
            .expect("future excluded")
            .unpriced_portfolios,
        1
    );
    let cached = prices(NOW - 1000, "3000000", "5000");
    assert!(cached.is_stale(NOW));
    assert_eq!(
        crypto_valuation(&after, Some(cached), NOW)
            .expect("offline")
            .assets,
        live.assets
    );
    app.execute(Command::SetCryptoHoldings {
        expected: after.accounts[0].account.clone(),
        holdings: holdings("0", "0"),
    })
    .expect("empty");
    let empty = crypto_valuation(&view(&mut app), None, NOW).expect("empty");
    assert_eq!(empty.unpriced_portfolios, 0);
    assert_eq!(empty.assets, Money::ZERO);
}

#[test]
fn converted_accounts_reject_cash_even_with_a_previously_prepared_entry() {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let account = legacy(&mut app);
    let input = EntryInput {
        amount: "80".into(),
        account: Some(account.id()),
        category: Some(Category::Food),
        ..EntryInput::empty(Today.today().expect("today"))
    };
    let Response::Prepared(prepared) = app
        .execute(Command::Preview(input.clone()))
        .expect("preview legacy")
    else {
        panic!("prepared")
    };
    app.execute(Command::SetCryptoHoldings {
        expected: account.clone(),
        holdings: holdings("1", "1"),
    })
    .expect("convert");
    assert!(app.execute(Command::Preview(input)).is_err());
    assert!(app.execute(Command::Commit(prepared)).is_err());
    assert!(
        app.execute(Command::AddRecurring(RecurringInput {
            name: "No crypto funding".into(),
            amount: "100".into(),
            day: "1".into(),
            start: "2026-09".into(),
            account: Some(account.id()),
            category: Some(Category::Food),
            installments: None
        }))
        .is_err()
    );
    assert_eq!(view(&mut app).entries.len(), 1);
}

#[test]
fn conversion_rejects_active_bill_links_and_deletion_cascades_only_own_holdings() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    let old = legacy(&mut app);
    app.execute(Command::AddRecurring(RecurringInput {
        name: "legacy bill".into(),
        amount: "100".into(),
        day: "1".into(),
        start: "2026-09".into(),
        account: Some(old.id()),
        category: Some(Category::Food),
        installments: None,
    }))
    .expect("bill");
    assert!(matches!(
        app.execute(Command::SetCryptoHoldings {
            expected: old,
            holdings: holdings("1", "0")
        }),
        Err(AppError::Storage(StorageError::CryptoRecurringAccount))
    ));
    let a = create(&mut app, "first", "1", "2");
    let b = create(&mut app, "second", "3", "4");
    let Response::AccountDeletion(plan) = app
        .execute(Command::PreviewDeleteAccount(a.id()))
        .expect("deletion")
    else {
        panic!("plan")
    };
    app.execute(Command::DeleteAccount(plan)).expect("delete");
    assert!(view(&mut app).accounts.iter().any(|v| v.account == b));
    let db = rusqlite::Connection::open(path).expect("inspect");
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM crypto_holding_changes WHERE account_id=?",
            [a.id().to_string()],
            |r| r.get::<_, i64>(0)
        )
        .expect("audit"),
        0
    );
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM crypto_holding_changes WHERE account_id=?",
            [b.id().to_string()],
            |r| r.get::<_, i64>(0)
        )
        .expect("other audit"),
        1
    );
}

#[test]
fn schema_nine_upgrade_does_not_reinterpret_fiat_as_coins() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("ledger.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    legacy(&mut app);
    let before = view(&mut app);
    drop(app);
    let db = rusqlite::Connection::open(&path).expect("v9 fixture");
    db.execute_batch("DROP TABLE crypto_holding_changes; DROP TABLE crypto_prices; ALTER TABLE accounts DROP COLUMN sol_atoms; ALTER TABLE accounts DROP COLUMN btc_atoms; PRAGMA user_version=9;").expect("v9");
    drop(db);
    let mut app = App::new(
        SqliteLedger::open(&path).expect("migrate"),
        Today,
        RandomIds,
    );
    assert_eq!(before, view(&mut app));
    assert!(
        view(&mut app).accounts[0]
            .account
            .crypto_holdings()
            .is_none()
    );
    assert!(matches!(
        app.execute(Command::LoadCryptoPrices).expect("no quote"),
        Response::CryptoPrices(None)
    ));
}

#[test]
fn native_portfolios_require_thb_and_overflow_is_an_error_not_a_zero_valuation() {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    app.execute(Command::SetCurrency(Currency::Usd))
        .expect("currency");
    assert!(
        app.execute(Command::CreateCryptoAccount {
            name: "usd".into(),
            holdings: holdings("1", "1")
        })
        .is_err()
    );
    assert!(view(&mut app).accounts.is_empty());
    app.execute(Command::SetCurrency(Currency::Thb))
        .expect("thb");
    create(&mut app, "huge", "92233720368.54775807", "0");
    assert!(
        crypto_valuation(
            &view(&mut app),
            Some(prices(NOW, "92233720368.54775807", "1")),
            NOW
        )
        .is_err()
    );
}
