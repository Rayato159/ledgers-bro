#![allow(clippy::expect_used)]
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::{RandomIds, SqliteLedger};
use tempfile::TempDir;
struct Today;
impl Clock for Today {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok("2026-09-24".parse()?)
    }
}
type App = LedgerApplication<SqliteLedger, Today, RandomIds>;
fn app(repo: SqliteLedger) -> App {
    LedgerApplication::new(repo, Today, RandomIds)
}
fn view(app: &mut App) -> Dashboard {
    let Response::Dashboard(view) = app.execute(Command::Load).expect("load") else {
        unreachable!()
    };
    view
}
#[test]
fn currency_persists_and_cannot_relabel_existing_or_deleted_account_history() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut a = app(SqliteLedger::open(&path).expect("open"));
    assert_eq!(view(&mut a).currency, Currency::Thb);
    a.execute(Command::SetCurrency(Currency::Usd))
        .expect("choose USD");
    assert!(!view(&mut a).thai_tax_enabled);
    assert!(a.execute(Command::SetThaiTaxEnabled(true)).is_err());
    a.execute(Command::CreateAccount {
        credit_cycle: None,
        name: "Cash THB".into(),
        kind: AccountKind::Cash,
        opening: "100.25".into(),
    })
    .expect("account");
    let before = view(&mut a);
    assert_eq!(before.assets.to_string(), "100.25");
    assert!(before.currency_locked);
    assert!(a.execute(Command::SetCurrency(Currency::Eur)).is_err());
    assert_eq!(view(&mut a), before);
    drop(a);
    let mut a = app(SqliteLedger::open(&path).expect("reopen"));
    assert_eq!(view(&mut a), before);
    for report in AccountingReport::ALL {
        for language in [ExportLanguage::English, ExportLanguage::Thai] {
            let Response::Csv(csv) = a
                .execute(Command::ExportCsv(ExportOptions { report, language }))
                .expect("export")
            else {
                unreachable!()
            };
            let header = csv.contents.lines().next().expect("header");
            assert!(header.contains("(USD)"));
            assert!(!header.contains("(THB)"));
            assert!(!header.contains("(บาท)"));
            assert!(csv.contents.contains("Cash THB"), "never rewrite names");
            assert!(csv.contents.contains("100.25"));
        }
    }
    let Response::AccountDeletion(plan) = a
        .execute(Command::PreviewDeleteAccount(
            before.accounts[0].account.id(),
        ))
        .expect("deletion preview")
    else {
        unreachable!()
    };
    a.execute(Command::DeleteAccount(plan))
        .expect("delete test account");
    assert!(
        a.execute(Command::SetCurrency(Currency::Gbp)).is_err(),
        "lock survives deletion"
    );
}
#[test]
fn explicit_wrong_units_and_receipts_never_become_foreign_ledger_amounts() {
    let mut a = app(SqliteLedger::in_memory().expect("db"));
    a.execute(Command::SetCurrency(Currency::Eur)).expect("EUR");
    a.execute(Command::CreateAccount {
        credit_cycle: None,
        name: "Cash".into(),
        kind: AccountKind::Cash,
        opening: "0".into(),
    })
    .expect("account");
    for prompt in [
        "จ่าย 80 บาท จาก Cash หมวด อาหาร",
        "จ่าย 80 USD จาก Cash หมวด อาหาร",
        "จ่าย $80 จาก Cash หมวด อาหาร",
        "จ่าย 80 JPY จาก Cash หมวด อาหาร",
        "[ใบเสร็จ 1]\nยอดสุทธิ: 100\n[จบใบเสร็จ]",
    ] {
        assert!(
            a.execute(Command::Resolve(prompt.into())).is_err(),
            "{prompt}"
        );
    }
    let Response::Resolved(QuickResolution::Draft { input, .. }) = a
        .execute(Command::Resolve("จ่าย 80.25 EUR จาก Cash หมวด อาหาร".into()))
        .expect("resolve EUR")
    else {
        unreachable!()
    };
    assert_eq!(input.amount, "80.25");
    let Response::Prepared(p) = a.execute(Command::Preview(input)).expect("preview") else {
        unreachable!()
    };
    a.execute(Command::Commit(p)).expect("commit");
    assert_eq!(view(&mut a).expenses.to_string(), "80.25");
    assert_eq!(
        normalize_prompt_currency(
            "จ่าย 9 EUR จาก \"USD travel\" หมวด อาหาร โน้ต \"$5 gift\"",
            Currency::Eur
        )
        .expect("quoted"),
        "จ่าย 9   จาก \"USD travel\" หมวด อาหาร โน้ต \"$5 gift\""
    );
    assert!(
        Currency::from_code("JPY").is_err(),
        "zero-decimal currencies need explicit support"
    );
}
#[test]
fn thai_tax_setting_persists_and_currency_switch_disables_it() {
    let mut a = app(SqliteLedger::in_memory().expect("db"));
    a.execute(Command::SetThaiTaxEnabled(false))
        .expect("disable");
    assert!(!view(&mut a).thai_tax_enabled);
    a.execute(Command::SetThaiTaxEnabled(true))
        .expect("enable for THB");
    assert!(view(&mut a).thai_tax_enabled);
    a.execute(Command::SetCurrency(Currency::Gbp)).expect("GBP");
    assert!(!view(&mut a).thai_tax_enabled);
    a.execute(Command::SetCurrency(Currency::Thb))
        .expect("empty ledger");
    assert!(
        !view(&mut a).thai_tax_enabled,
        "tax isn't re-enabled implicitly"
    );
}
#[test]
fn v5_upgrade_preserves_thb_amounts_and_locks_existing_ledgers() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut a = app(SqliteLedger::open(&path).expect("open"));
    a.execute(Command::CreateAccount {
        credit_cycle: None,
        name: "เงินสด".into(),
        kind: AccountKind::Cash,
        opening: "456.78".into(),
    })
    .expect("account");
    let expected = view(&mut a);
    drop(a);
    let raw = rusqlite::Connection::open(&path).expect("raw");
    raw.execute_batch("ALTER TABLE accounts DROP COLUMN payment_day; ALTER TABLE accounts DROP COLUMN closing_day;").expect("remove v8 columns for old schema fixture");
    raw.execute_batch("DROP TABLE user_preferences; DROP TRIGGER lock_currency_accounts; DROP TRIGGER lock_currency_recurring; DROP TRIGGER lock_currency_receivables; DROP TABLE ledger_settings; PRAGMA user_version=5;").expect("v5 fixture");
    let mut a = app(SqliteLedger::open(&path).expect("upgrade"));
    assert_eq!(view(&mut a), expected);
    assert!(a.execute(Command::SetCurrency(Currency::Usd)).is_err());
}

#[test]
fn display_preferences_survive_restart_without_changing_currency_or_balances() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut ledger = app(SqliteLedger::open(&path).expect("open"));
    ledger
        .execute(Command::SetCurrency(Currency::Eur))
        .expect("EUR");
    let before = view(&mut ledger);
    let custom = UserPreferences {
        english: true,
        dark: true,
        primary_color: 0x2376ae,
        gradient: false,
    };
    assert!(
        matches!(ledger.execute(Command::SetPreferences(custom)).expect("save"), Response::Preferences(p) if p == custom)
    );
    assert!(
        ledger
            .execute(Command::SetPreferences(UserPreferences {
                primary_color: 0x1000000,
                ..custom
            }))
            .is_err()
    );
    drop(ledger);
    let mut reopened = app(SqliteLedger::open(&path).expect("reopen"));
    assert!(
        matches!(reopened.execute(Command::LoadPreferences).expect("load"), Response::Preferences(p) if p == custom)
    );
    let after = view(&mut reopened);
    assert_eq!(after.currency, before.currency);
    assert_eq!(after.net_worth, before.net_worth);
    assert_eq!(after.accounts, before.accounts);
    assert_eq!(after.entries, before.entries);
    assert!(!after.thai_tax_enabled);
}

#[test]
fn v6_migration_adds_preferences_without_resetting_ledger_settings() {
    let dir = TempDir::new().expect("temp");
    let path = dir.path().join("ledger.sqlite3");
    let mut ledger = app(SqliteLedger::open(&path).expect("open"));
    ledger
        .execute(Command::SetCurrency(Currency::Gbp))
        .expect("GBP");
    drop(ledger);
    let raw = rusqlite::Connection::open(&path).expect("raw");
    raw.execute_batch("ALTER TABLE accounts DROP COLUMN payment_day; ALTER TABLE accounts DROP COLUMN closing_day;").expect("remove v8 columns for old schema fixture");
    raw.execute_batch("DROP TABLE user_preferences; PRAGMA user_version=6;")
        .expect("v6 fixture");
    let mut migrated = app(SqliteLedger::open(&path).expect("migrate"));
    assert!(
        matches!(migrated.execute(Command::LoadPreferences).expect("load"), Response::Preferences(p) if p == UserPreferences::default())
    );
    assert_eq!(view(&mut migrated).currency, Currency::Gbp);
    assert!(!view(&mut migrated).thai_tax_enabled);
}
