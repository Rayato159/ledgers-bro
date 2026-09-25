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
        panic!("view")
    };
    v
}
fn account(app: &mut App, name: &str, kind: AccountKind) -> AccountId {
    app.execute(Command::CreateAccount {
        name: name.into(),
        kind,
        opening: "1000".into(),
        credit_cycle: if kind == AccountKind::CreditCard {
            Some(CreditCardCycle::new(20, 5).expect("cycle"))
        } else {
            None
        },
    })
    .expect("account");
    view(app)
        .accounts
        .iter()
        .find(|a| a.account.name().as_str() == name)
        .expect("find")
        .account
        .id()
}
fn preview(app: &mut App, text: &str) -> Vec<PreparedEntry> {
    let Response::PreparedCsv(entries) = app
        .execute(Command::PreviewCsvImport(text.as_bytes().into()))
        .expect("preview")
    else {
        panic!("preview")
    };
    entries
}
fn setup() -> App {
    let mut app = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    account(&mut app, "เงินสด", AccountKind::Cash);
    account(&mut app, "ธนาคาร", AccountKind::Bank);
    app
}
fn template() -> String {
    csv_template("2026-09-25".parse().expect("date"), Currency::Thb).contents
}

#[test]
fn csv_template_imports_thai_atomically_and_identical_retry_does_not_duplicate() {
    let mut app = setup();
    let before = view(&mut app);
    let entries = preview(&mut app, &template());
    assert_eq!(entries.len(), 3);
    assert_eq!(view(&mut app), before, "preview never writes");
    let Response::CommittedBatch(first) = app
        .execute(Command::CommitCsvImport(entries.clone()))
        .expect("import")
    else {
        panic!("outcomes")
    };
    assert!(first.iter().all(|o| matches!(o, CommitOutcome::Saved(_))));
    let after = view(&mut app);
    assert_eq!(after.entries.len(), before.entries.len() + 3);
    let entries = preview(&mut app, &template());
    let Response::CommittedBatch(second) = app
        .execute(Command::CommitCsvImport(entries))
        .expect("retry")
    else {
        panic!("outcomes")
    };
    assert!(
        second
            .iter()
            .all(|o| matches!(o, CommitOutcome::AlreadySaved(_)))
    );
    assert_eq!(view(&mut app), after);
    assert!(
        app.execute(Command::PreviewCsvImport(
            template().replace("80.00", "81.00").into_bytes().into()
        ))
        .is_err()
    );
    assert_eq!(view(&mut app), after);
}

#[test]
fn malformed_unsupported_and_stale_imports_never_partially_write() {
    let mut app = setup();
    let original = view(&mut app);
    let source = template();
    for text in [
        source.replace("external_id", "id"),
        source.replace("THB", "USD"),
        source.replace("expense", "other"),
        source.replace("ธนาคาร", "ไม่มีบัญชี"),
        source.replace("80.00", "80.001"),
        source.replace("2026-09-25", "2026-09-26"),
        source.replace("coffee-001", "salary-001"),
        source.replace("กาแฟ", "\"broken"),
        source.replace("กาแฟ", "cof\"fee"),
        source.replace("food", "salary"),
    ] {
        assert!(
            app.execute(Command::PreviewCsvImport(text.into_bytes().into()))
                .is_err()
        );
        assert_eq!(view(&mut app), original);
    }
    let quoted = source.replace("กาแฟ", "\"กาแฟ, นม\nมื้อเช้า\"");
    assert_eq!(
        preview(&mut app, &quoted)[0].entry.note().as_str(),
        "กาแฟ, นม\nมื้อเช้า"
    );
    let mut entries = preview(&mut app, &source);
    entries[1].submission = entries[0].submission;
    assert!(app.execute(Command::CommitCsvImport(entries)).is_err());
    assert_eq!(view(&mut app), original);
    assert!(
        app.execute(Command::PreviewCsvImport(vec![255u8].into()))
            .is_err()
    );
}

#[test]
fn logical_backup_roundtrip_preserves_tax_card_crypto_plans_preferences_and_csv_idempotency() {
    let dir = tempfile::tempdir().expect("dir");
    let path = dir.path().join("source.sqlite3");
    let mut app = App::new(SqliteLedger::open(&path).expect("db"), Today, RandomIds);
    account(&mut app, "เงินสด", AccountKind::Cash);
    let bank = account(&mut app, "ธนาคาร", AccountKind::Bank);
    let card = account(&mut app, "บัตร", AccountKind::CreditCard);
    let imported = preview(&mut app, &template());
    app.execute(Command::CommitCsvImport(imported))
        .expect("csv");
    app.execute(Command::AddRecurring(RecurringInput {
        name: "Discord".into(),
        amount: "215".into(),
        day: "1".into(),
        start: "2026-10".into(),
        account: Some(card),
        category: Some(Category::Luxury),
        installments: Some("12".into()),
    }))
    .expect("plan");
    app.execute(Command::CreateCryptoAccount {
        name: "พอร์ต".into(),
        holdings: CryptoHoldings::parse("0.12345678", "1.123456789").expect("coins"),
    })
    .expect("crypto");
    let mut income = EntryInput::empty("2026-09-25".parse().expect("date"));
    income.kind = TransactionKind::Income;
    income.amount = "10400".into();
    income.account = Some(bank);
    income.category = Some(Category::Freelance);
    income.income_tax = Some(Box::new(IncomeTaxInput {
        section: Some(IncomeSection::new(2).expect("section")),
        gross: "10000".into(),
        vat: "700".into(),
        withholding: "300".into(),
        other_deductions: "0".into(),
    }));
    let Response::Prepared(income) = app.execute(Command::Preview(income)).expect("income") else {
        panic!("entry")
    };
    app.execute(Command::Commit(income)).expect("save");
    let preferences = UserPreferences {
        dark: true,
        english: true,
        primary_color: 0x60c5a2,
        gradient: false,
    };
    app.execute(Command::SetPreferences(preferences))
        .expect("preferences");
    let before = view(&mut app);
    let Response::Document(file) = app.execute(Command::ExportBackup).expect("export") else {
        panic!("file")
    };
    assert!(
        std::str::from_utf8(&file.bytes)
            .expect("utf8")
            .contains("ธนาคาร")
    );
    let mut dest = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let Response::BackupReview(review) = dest
        .execute(Command::PreviewBackup(file.bytes.clone()))
        .expect("review")
    else {
        panic!("review")
    };
    assert_eq!(review.summary.accounts, 4);
    assert_eq!(review.summary.plans, 1);
    assert!(view(&mut dest).accounts.is_empty());
    dest.execute(Command::RestoreBackup(review.clone()))
        .expect("restore");
    assert_eq!(view(&mut dest), before);
    let Response::Preferences(saved) = dest.execute(Command::LoadPreferences).expect("prefs")
    else {
        panic!("prefs")
    };
    assert_eq!(saved, preferences);
    let entries = preview(&mut dest, &template());
    dest.execute(Command::CommitCsvImport(entries))
        .expect("retry");
    assert_eq!(view(&mut dest), before);
    assert!(dest.execute(Command::RestoreBackup(review)).is_err());
    assert_eq!(view(&mut dest), before);
    let Response::Document(exported) = dest.execute(Command::ExportBackup).expect("reexport")
    else {
        panic!("file")
    };
    assert_eq!(file.bytes, exported.bytes);
}

#[test]
fn backup_rejects_foreign_schema_unknown_tables_and_unbalanced_postings_without_overwrite() {
    let mut source = setup();
    let Response::Document(file) = source.execute(Command::ExportBackup).expect("export") else {
        panic!("file")
    };
    let mut destination = App::new(SqliteLedger::in_memory().expect("db"), Today, RandomIds);
    let before = view(&mut destination);
    let original: serde_json::Value = serde_json::from_slice(&file.bytes).expect("json");
    for change in 0..5 {
        let mut value = original.clone();
        match change {
            0 => value["schema"] = 999.into(),
            1 => value["tables"][2]["name"] = "profiles; DROP TABLE accounts".into(),
            2 => value["tables"][6]["rows"][0][4]["value"] = 9.into(),
            3 => value["sql"] = "DROP TABLE accounts".into(),
            _ => value["tables"][2]["columns"][0] = "other".into(),
        }
        let bytes: std::sync::Arc<[u8]> = serde_json::to_vec(&value).expect("encode").into();
        assert!(
            destination
                .execute(Command::PreviewBackup(bytes.clone()))
                .is_err()
        );
        assert!(
            destination
                .execute(Command::RestoreBackup(BackupReview {
                    bytes,
                    summary: BackupSummary {
                        accounts: 0,
                        transactions: 0,
                        plans: 0,
                        receivables: 0,
                        currency: Currency::Thb
                    }
                }))
                .is_err()
        );
        assert_eq!(view(&mut destination), before);
    }
}
