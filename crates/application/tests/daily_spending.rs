#![allow(clippy::expect_used)]
use ledger_application::*;
use ledger_domain::*;

fn id(n: usize) -> EntryId {
    format!("00000000-0000-0000-0000-{n:012}")
        .parse()
        .expect("id")
}
fn account() -> AccountId {
    "00000000-0000-0000-0000-000000000001"
        .parse()
        .expect("account")
}
fn expense(n: usize, date: &str, amount: &str) -> JournalEntry {
    JournalEntry::record(
        id(n),
        date.parse().expect("date"),
        Note::new("Synthetic expense").expect("note"),
        EntryKind::Expense {
            account: account(),
            amount: PositiveMoney::new(amount.parse().expect("money")).expect("positive"),
            category: Category::Food,
        },
    )
    .expect("entry")
}
fn view(date: &str) -> Dashboard {
    dashboard(
        LedgerState {
            currency: Currency::Thb,
            currency_locked: false,
            thai_tax_enabled: false,
            receivables: vec![],
            recurring: vec![],
            settlements: vec![],
            accounts: vec![],
            entries: vec![],
        },
        date.parse().expect("date"),
    )
    .expect("dashboard")
}
#[test]
fn seven_calendar_days_cross_year_include_today_and_zero_days() {
    let mut view = view("2026-01-03");
    view.entries = vec![
        expense(1, "2025-12-27", "999"),
        expense(2, "2025-12-28", "10"),
        expense(3, "2025-12-31", "20"),
        expense(4, "2026-01-03", "0.10"),
        expense(5, "2026-01-03", "0.20"),
        expense(6, "2026-01-04", "999"),
    ];
    let report = daily_spending(&view).expect("report");
    assert_eq!(
        report
            .days
            .iter()
            .map(|d| d.date.to_string())
            .collect::<Vec<_>>(),
        [
            "2025-12-28",
            "2025-12-29",
            "2025-12-30",
            "2025-12-31",
            "2026-01-01",
            "2026-01-02",
            "2026-01-03"
        ]
    );
    assert_eq!(
        report
            .days
            .iter()
            .map(|d| d.amount.minor())
            .collect::<Vec<_>>(),
        [1000, 0, 0, 2000, 0, 0, 30]
    );
    assert_eq!(report.days[6].count, 2);
    assert_eq!(report.total.minor(), 3030);
}
#[test]
fn ignores_income_transfers_openings_reversals_and_cancelled_expenses() {
    let mut view = view("2026-09-28");
    let cancelled = expense(1, "2026-09-28", "100");
    let reversal = JournalEntry::reverse(
        id(2),
        &cancelled,
        Note::new("Synthetic cancellation").expect("note"),
    )
    .expect("reversal");
    view.reversed.insert(cancelled.id());
    view.entries = vec![cancelled, reversal, expense(3, "2026-09-28", "15")];
    for (n, kind) in [
        (
            4,
            EntryKind::Income {
                account: account(),
                amount: PositiveMoney::new("500".parse().expect("money")).expect("positive"),
                category: Category::OtherIncome,
            },
        ),
        (
            5,
            EntryKind::Opening {
                account: account(),
                balance: "999".parse().expect("money"),
            },
        ),
        (
            6,
            EntryKind::Transfer {
                from: account(),
                to: "00000000-0000-0000-0000-000000000002"
                    .parse()
                    .expect("other"),
                amount: PositiveMoney::new("50".parse().expect("money")).expect("positive"),
            },
        ),
    ] {
        view.entries.push(
            JournalEntry::record(id(n), view.today, Note::new("").expect("note"), kind)
                .expect("entry"),
        );
    }
    let report = daily_spending(&view).expect("report");
    assert_eq!(report.total.minor(), 1500);
    assert_eq!(report.days[6].count, 1);
}
#[test]
fn handles_leap_day_empty_ledgers_and_earliest_domain_date_without_panicking() {
    let report = daily_spending(&view("2024-03-02")).expect("leap");
    assert_eq!(report.days[4].date.to_string(), "2024-02-29");
    assert!(
        report
            .days
            .iter()
            .all(|d| d.amount == Money::ZERO && d.count == 0)
    );
    assert_eq!(
        daily_spending(&view("1900-01-01"))
            .expect("boundary")
            .days
            .len(),
        7
    );
}
#[test]
fn rejects_aggregate_overflow_instead_of_wrapping_or_rounding_money() {
    let mut view = view("2026-02-01");
    view.entries = vec![
        expense(1, "2026-01-31", "90000000000"),
        expense(2, "2026-02-01", "0.01"),
    ];
    assert_eq!(daily_spending(&view), Err(DomainError::MoneyOverflow));
}
