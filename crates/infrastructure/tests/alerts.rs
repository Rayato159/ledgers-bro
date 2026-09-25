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
    let prefs = AlertPreferences::default();
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
    assert!(
        ledger_alerts(&dashboard(state.clone(), today).expect("view"), &prefs)
            .expect("paid")
            .is_empty()
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
