#![allow(clippy::expect_used, clippy::panic)]
use ledger_application::*;
use ledger_domain::*;

fn account_id(n: u32) -> AccountId {
    format!("00000000-0000-0000-0000-{n:012}")
        .parse()
        .expect("account")
}
fn entry_id(n: u32) -> EntryId {
    format!("10000000-0000-0000-0000-{n:012}")
        .parse()
        .expect("entry")
}
fn amount(text: &str) -> PositiveMoney {
    PositiveMoney::new(text.parse().expect("money")).expect("positive")
}
fn entry(n: u32, kind: EntryKind) -> JournalEntry {
    JournalEntry::record(
        entry_id(n),
        "2026-09-20".parse().expect("date"),
        Note::new("รายการทดสอบ").expect("note"),
        kind,
    )
    .expect("entry")
}
fn fixture() -> Dashboard {
    let cash = account_id(1);
    let bank = account_id(2);
    let card = account_id(3);
    let accounts = [
        (cash, "เงินสด", AccountKind::Cash),
        (bank, "ธนาคาร", AccountKind::Bank),
        (card, "บัตรเครดิต", AccountKind::CreditCard),
    ]
    .map(|(id, name, kind)| Account::new(id, AccountName::new(name).expect("name"), kind))
    .to_vec();
    let mut entries = vec![
        entry(
            1,
            EntryKind::Opening {
                account: cash,
                balance: "1000".parse().expect("money"),
            },
        ),
        entry(
            2,
            EntryKind::Opening {
                account: bank,
                balance: Money::ZERO,
            },
        ),
        entry(
            3,
            EntryKind::Opening {
                account: card,
                balance: "-50".parse().expect("money"),
            },
        ),
        entry(
            4,
            EntryKind::Income {
                account: bank,
                amount: amount("300"),
                category: Category::Salary,
            },
        ),
        entry(
            5,
            EntryKind::Expense {
                account: cash,
                amount: amount("80.25"),
                category: Category::Food,
            },
        ),
        entry(
            6,
            EntryKind::Expense {
                account: card,
                amount: amount("100"),
                category: Category::Supplies,
            },
        ),
        entry(
            7,
            EntryKind::Transfer {
                from: bank,
                to: card,
                amount: amount("100"),
            },
        ),
        entry(
            8,
            EntryKind::Expense {
                account: cash,
                amount: amount("20"),
                category: Category::Food,
            },
        ),
    ];
    entries.push(
        JournalEntry::reverse(
            entry_id(9),
            &entries[7],
            Note::new("ยกเลิกรายการโดยผู้ใช้").expect("note"),
        )
        .expect("reversal"),
    );
    dashboard(
        LedgerState { accounts, entries },
        "2026-09-20".parse().expect("today"),
    )
    .expect("dashboard")
}
fn export(view: &Dashboard, report: AccountingReport, language: ExportLanguage) -> CsvExport {
    export_csv(view, ExportOptions { report, language }).expect("export")
}
// These arithmetic fixtures intentionally have no quoting characters. Actual
// UTF-8, quotes, commas and multiline round trips use an independent CSV reader
// in scripts/verify-accounting-exports.ps1 against generated sample files.
fn simple_rows(contents: &str) -> Vec<Vec<String>> {
    contents
        .trim_start_matches('\u{feff}')
        .lines()
        .map(|line| {
            line.split(',')
                .map(|cell| cell.trim_matches('"').to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn journal_uses_real_debits_credits_and_keeps_originals_and_reversals() {
    let file = export(&fixture(), AccountingReport::Journal, ExportLanguage::Thai);
    let rows = simple_rows(&file.contents);
    assert_eq!(rows.len(), 20); // header + 9 entries x 2 postings + total
    let posting = |n: u32, account: &str| {
        rows.iter()
            .find(|r| r[1] == entry_id(n).to_string() && r[5] == account)
            .expect("posting")
    };
    assert_eq!(&posting(5, "expense:food")[7..9], &["80.25", "0.00"]);
    assert_eq!(
        &posting(5, &account_id(1).to_string())[7..9],
        &["0.00", "80.25"]
    );
    assert_eq!(
        &posting(3, &account_id(3).to_string())[7..9],
        &["0.00", "50.00"],
        "opening debt is a credit"
    );
    assert_eq!(&posting(4, "income:salary")[7..9], &["0.00", "300.00"]);
    assert_eq!(
        &posting(7, &account_id(3).to_string())[7..9],
        &["100.00", "0.00"],
        "payment reduces debt by debiting card"
    );
    assert_eq!(
        &posting(7, &account_id(2).to_string())[7..9],
        &["0.00", "100.00"]
    );
    assert_eq!(&posting(9, "expense:food")[7..9], &["0.00", "20.00"]);
    assert_eq!(posting(9, "expense:food")[4], entry_id(8).to_string());
    assert_eq!(posting(8, "expense:food")[3], "ถูกกลับรายการแล้ว");
    assert_eq!(&rows.last().expect("total")[7..9], &["1670.25", "1670.25"]);
    for pair in rows[1..19].chunks_exact(2) {
        assert_eq!(pair[0][1], pair[1][1]);
        assert_eq!(pair[0][7], pair[1][8], "debit first and equal credit");
        assert_eq!(pair[0][8], "0.00");
        assert_eq!(pair[1][7], "0.00");
    }
}

#[test]
fn trial_balance_reconciles_to_independently_calculated_balances() {
    let file = export(
        &fixture(),
        AccountingReport::TrialBalance,
        ExportLanguage::Thai,
    );
    let rows = simple_rows(&file.contents);
    for (id, dr, cr) in [
        (account_id(1).to_string(), "919.75", "0.00"),
        (account_id(2).to_string(), "200.00", "0.00"),
        (account_id(3).to_string(), "0.00", "50.00"),
        ("equity:opening".into(), "0.00", "950.00"),
        ("income:salary".into(), "0.00", "300.00"),
        ("expense:food".into(), "80.25", "0.00"),
        ("expense:supplies".into(), "100.00", "0.00"),
    ] {
        let row = rows.iter().find(|r| r[1] == id).expect("account");
        assert_eq!(&row[5..7], &[dr, cr]);
    }
    assert_eq!(
        &rows.last().expect("total")[3..7],
        &["1670.25", "1670.25", "1300.00", "1300.00"]
    );
}

#[test]
fn general_ledger_groups_accounts_and_retains_running_debit_and_credit_balances() {
    let file = export(
        &fixture(),
        AccountingReport::GeneralLedger,
        ExportLanguage::English,
    );
    let rows = simple_rows(&file.contents);
    let cash: Vec<_> = rows
        .iter()
        .filter(|r| r[5] == account_id(1).to_string())
        .collect();
    assert_eq!(
        cash.iter().map(|r| r[10].as_str()).collect::<Vec<_>>(),
        vec!["1000.00", "919.75", "899.75", "919.75"]
    );
    let card: Vec<_> = rows
        .iter()
        .filter(|r| r[5] == account_id(3).to_string())
        .collect();
    assert_eq!(
        card.iter().map(|r| r[11].as_str()).collect::<Vec<_>>(),
        vec!["50.00", "150.00", "50.00"]
    );
    assert_eq!(
        &rows.last().expect("total")[10..12],
        &["", ""],
        "running balances must not be summed as activity"
    );
}

#[test]
fn thai_is_default_and_english_localizes_system_text_without_translating_user_notes() {
    let view = fixture();
    let thai = export_csv(&view, ExportOptions::default()).expect("Thai export");
    assert!(thai.filename.starts_with("สมุดรายวันทั่วไป-"));
    assert!(thai.contents.as_bytes().starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(thai.contents.contains("\"เดบิต (บาท)\",\"เครดิต (บาท)\""));
    assert!(thai.contents.contains("รายได้ — เงินเดือน"));
    assert!(!thai.contents.contains("\"Posted\"") && !thai.contents.contains("\"cancelled\""));
    let english = export(&view, AccountingReport::Journal, ExportLanguage::English);
    assert!(english.filename.starts_with("General journal-"));
    assert!(
        english
            .contents
            .contains("\"Debit (THB)\",\"Credit (THB)\"")
    );
    assert!(english.contents.contains("Salary income"));
    assert!(english.contents.contains("Cancelled by user"));
    assert!(
        english.contents.contains("รายการทดสอบ"),
        "user-authored text is preserved"
    );
}

#[test]
fn empty_reports_have_headers_and_balanced_zero_totals() {
    let empty =
        dashboard(LedgerState::default(), "2026-09-20".parse().expect("date")).expect("empty");
    for report in AccountingReport::ALL {
        let file = export(&empty, report, ExportLanguage::Thai);
        let rows = simple_rows(&file.contents);
        assert_eq!(rows.len(), 2);
        assert!(rows[1].contains(&"รวม".to_owned()));
        assert!(rows[1].contains(&"0.00".to_owned()));
    }
}

#[test]
fn all_history_exports_in_date_order_with_activity_totals_wider_than_money_balances() {
    let cash = account_id(1);
    let account = Account::new(
        cash,
        AccountName::new("เงินสด").expect("name"),
        AccountKind::Cash,
    );
    let mut entries = Vec::new();
    for (n, date, kind) in [
        (
            1,
            "2026-09-01",
            EntryKind::Income {
                account: cash,
                amount: amount("90000000000"),
                category: Category::Salary,
            },
        ),
        (
            2,
            "2026-09-01",
            EntryKind::Expense {
                account: cash,
                amount: amount("90000000000"),
                category: Category::Food,
            },
        ),
        (
            3,
            "2026-08-01",
            EntryKind::Income {
                account: cash,
                amount: amount("90000000000"),
                category: Category::Salary,
            },
        ),
        (
            4,
            "2026-08-01",
            EntryKind::Expense {
                account: cash,
                amount: amount("90000000000"),
                category: Category::Food,
            },
        ),
    ] {
        entries.push(
            JournalEntry::record(
                entry_id(n),
                date.parse().expect("date"),
                Note::new("").expect("note"),
                kind,
            )
            .expect("entry"),
        );
    }
    let view = dashboard(
        LedgerState {
            accounts: vec![account],
            entries,
        },
        "2026-09-20".parse().expect("date"),
    )
    .expect("view");
    let file = export(&view, AccountingReport::Journal, ExportLanguage::Thai);
    let rows = simple_rows(&file.contents);
    assert_eq!(rows[1][0], "2026-08-01");
    assert_eq!(rows[5][0], "2026-09-01");
    assert_eq!(
        &rows.last().expect("total")[7..9],
        &["360000000000.00", "360000000000.00"]
    );
}

#[test]
fn missing_account_or_original_never_produces_a_plausible_accounting_file() {
    let mut view = fixture();
    view.accounts.clear();
    assert_eq!(
        export_csv(&view, ExportOptions::default()).expect_err("missing account"),
        DomainError::AccountUnavailable
    );
    let mut view = fixture();
    view.entries.retain(|e| e.id() != entry_id(8));
    assert_eq!(
        export_csv(&view, ExportOptions::default()).expect_err("missing original"),
        DomainError::InvalidReversal
    );
}
