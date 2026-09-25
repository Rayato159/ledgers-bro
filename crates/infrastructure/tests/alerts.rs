#![allow(clippy::expect_used)]
use futures_executor::block_on;
use ledger_application::*;
use ledger_domain::*;
use ledger_infrastructure::AlertStore;

#[test]
fn reminders_follow_dates_payments_reversals_and_preferences() {
    let today: EntryDate = "2026-12-30".parse().expect("today");
    let account = Account::new(
        "00000000-0000-0000-0000-000000000001".parse().expect("id"),
        AccountName::new("Synthetic cash").expect("name"),
        AccountKind::Cash,
    );
    let schedule = RecurringInput {
        name: "Synthetic rent".into(),
        amount: "100".into(),
        day: "1".into(),
        start: "2027-01".into(),
        account: Some(account.id()),
        category: Some(Category::Rent),
        installments: Some("1".into()),
    }
    .validate("00000000-0000-0000-0000-000000000002".parse().expect("id"))
    .expect("plan");
    let mut state = LedgerState {
        accounts: vec![account.clone()],
        recurring: vec![schedule.clone()],
        ..Default::default()
    };
    let mut prefs = AlertPreferences::default();
    let view = dashboard(state.clone(), today).expect("view");
    let alerts = ledger_alerts(&view, &prefs).expect("alerts");
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].due.expect("due").to_string(), "2027-01-01");
    assert!(
        ledger_alerts(
            &dashboard(state.clone(), "2026-12-28".parse().expect("date")).expect("view"),
            &prefs
        )
        .expect("early")
        .is_empty()
    );
    assert!(
        ledger_alerts(
            &view,
            &AlertPreferences {
                bills: false,
                ..prefs.clone()
            }
        )
        .expect("off")
        .is_empty()
    );
    let payment = JournalEntry::record(
        "00000000-0000-0000-0000-000000000003".parse().expect("id"),
        today,
        Note::new("Synthetic prepaid bill").expect("note"),
        EntryKind::Expense {
            account: account.id(),
            amount: PositiveMoney::new("100".parse().expect("money")).expect("positive"),
            category: Category::Rent,
        },
    )
    .expect("entry");
    state.settlements.push(RecurringSettlement {
        recurring: schedule.id(),
        month: "2027-01".parse().expect("month"),
        entry: payment.id(),
    });
    state.entries.push(payment.clone());
    prefs.archive(&alerts[0]);
    assert!(
        ledger_alerts(&dashboard(state.clone(), today).expect("view"), &prefs)
            .expect("paid")
            .is_empty()
    );
    let dir = tempfile::tempdir().expect("archive dir");
    let archive =
        AlertStore::new(dir.path(), "00000000-0000-0000-0000-000000000001").expect("archive");
    block_on(archive.save(prefs.clone())).expect("save paid history");
    let restored = block_on(archive.load()).expect("reload history");
    assert!(restored.is_read(&alerts[0].key));
    assert_eq!(
        restored.archived[0].alert().expect("archived snapshot"),
        alerts[0]
    );
    state.entries.push(
        JournalEntry::reverse(
            "00000000-0000-0000-0000-000000000004".parse().expect("id"),
            &payment,
            Note::new("Synthetic reversal").expect("note"),
        )
        .expect("reversal"),
    );
    assert_eq!(
        ledger_alerts(&dashboard(state, today).expect("view"), &prefs)
            .expect("reopened")
            .len(),
        1
    );
}

#[test]
fn alert_preferences_are_bounded_and_isolated_between_users() {
    let dir = tempfile::tempdir().expect("dir");
    let one = AlertStore::new(dir.path(), "00000000-0000-0000-0000-000000000001").expect("one");
    let two = AlertStore::new(dir.path(), "00000000-0000-0000-0000-000000000002").expect("two");
    let mut prefs = AlertPreferences::default();
    for i in 0..5000 {
        prefs.mark_read(&format!("bill:{i}"));
        prefs.mark_notified(&format!("2026-09-26:bill:{i}"));
    }
    assert!(prefs.valid());
    assert_eq!(prefs.read.len(), 2048);
    block_on(one.save(prefs.clone())).expect("save");
    assert_eq!(block_on(one.load()).expect("load"), prefs);
    assert_eq!(
        block_on(two.load()).expect("other user"),
        AlertPreferences::default()
    );
    assert!(AlertStore::new(dir.path(), "../outside").is_err());
}

#[test]
fn archives_keep_original_snapshots_deduplicate_and_limit_history() {
    let mut prefs = AlertPreferences::default();
    let mut alert = LedgerAlert {
        key: "bill:original".into(),
        kind: AlertKind::Bill,
        title: "Synthetic bill".into(),
        due: Some("2026-09-26".parse().expect("date")),
        amount: Some("123.45".parse().expect("money")),
    };
    prefs.archive(&alert);
    let original = prefs.archived[0].clone();
    alert.title = "Changed after it was read".into();
    alert.amount = Some(Money::ZERO);
    prefs.archive(&alert);
    assert_eq!(prefs.archived, vec![original]);
    for i in 0..300 {
        alert.key = format!("bill:{i}");
        prefs.archive(&alert);
    }
    assert!(prefs.valid());
    assert_eq!(prefs.archived.len(), AlertPreferences::ARCHIVE_LIMIT);
    assert_eq!(prefs.archived[0].key, "bill:299");
    assert_eq!(prefs.archived.last().expect("oldest").key, "bill:44");
    assert!(prefs.is_read("bill:original"));
    assert!(!prefs.is_read("bill:new-month"));
}

#[test]
fn legacy_preferences_load_and_archives_are_profile_isolated() {
    let dir = tempfile::tempdir().expect("dir");
    let id = "00000000-0000-0000-0000-000000000001";
    std::fs::write(
        dir.path().join(format!("notifications-{id}.json")),
        r#"{"bills":false,"read":["update:0.1.5"],"notified":[]}"#,
    )
    .expect("legacy");
    let one = AlertStore::new(dir.path(), id).expect("one");
    let two = AlertStore::new(dir.path(), "00000000-0000-0000-0000-000000000002").expect("two");
    let mut prefs = block_on(one.load()).expect("legacy load");
    assert!(!prefs.bills);
    assert!(prefs.archived.is_empty());
    assert!(prefs.is_read("update:0.1.5"));
    prefs.archive(&LedgerAlert {
        key: "update:0.1.5".into(),
        kind: AlertKind::Update,
        title: "Ledgers Bro 0.1.5".into(),
        due: None,
        amount: None,
    });
    block_on(one.save(prefs.clone())).expect("save");
    assert_eq!(block_on(one.load()).expect("reload"), prefs);
    assert!(
        block_on(two.load())
            .expect("other profile")
            .archived
            .is_empty()
    );
    prefs.archived[0].due = Some("2026-99-99".into());
    assert!(block_on(one.save(prefs)).is_err());
    assert!(
        block_on(one.load())
            .expect("failed save keeps file")
            .archived[0]
            .due
            .is_none()
    );
}
