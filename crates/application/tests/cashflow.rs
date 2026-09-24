#![allow(clippy::expect_used)]
use ledger_application::*;
use ledger_domain::*;
fn flow(income: &str, expenses: &str) -> MonthlyFlow {
    MonthlyFlow {
        year: 2026,
        month: 1,
        income: income.parse().expect("income"),
        expenses: expenses.parse().expect("expense"),
    }
}
#[test]
fn rate_share_comparison_and_health_have_defined_zero_and_boundary_behavior() {
    for (income, expense, rate, health) in [
        ("0", "0", None, FlowHealth::NoData),
        ("1", "0", Some(10000), FlowHealth::Good),
        ("0", "1", Some(-10000), FlowHealth::Poor),
        ("100", "100", Some(0), FlowHealth::Fair),
        ("110", "90", Some(1000), FlowHealth::Good),
        ("109.99", "90.01", Some(999), FlowHealth::Fair),
        ("99.99", "100", Some(0), FlowHealth::Poor),
        ("400", "30", Some(8604), FlowHealth::Good),
    ] {
        let flow = flow(income, expense);
        assert_eq!(flow.rate(), rate);
        assert_eq!(flow.health(), health);
    }
    assert_eq!(flow("400", "30").income_share(), Some(9302));
    assert_eq!(flow("400", "30").difference_percent(), Some(123333));
    assert_eq!(flow("30", "400").difference_percent(), Some(123333));
    assert_eq!(flow("0", "30").difference_percent(), None);
    assert_eq!(flow("0", "0").income_share(), None);
    assert_eq!(flow("90000000000", "90000000000").rate(), Some(0));
    assert_eq!(flow("90000000000", "0.01").rate(), Some(9999));
}
#[test]
fn six_months_cross_year_and_ignore_transfers_openings_future_and_reversals() {
    let a: AccountId = "00000000-0000-0000-0000-000000000001".parse().expect("a");
    let b: AccountId = "00000000-0000-0000-0000-000000000002".parse().expect("b");
    let positive =
        |amount: &str| PositiveMoney::new(amount.parse().expect("money")).expect("positive");
    let record = |id: usize, date: &str, kind| {
        JournalEntry::record(
            format!("00000000-0000-0000-0000-{id:012}")
                .parse()
                .expect("id"),
            date.parse().expect("date"),
            Note::new("").expect("note"),
            kind,
        )
        .expect("entry")
    };
    let mut state = LedgerState {
        currency: Currency::Thb,
        currency_locked: false,
        thai_tax_enabled: true,
        receivables: vec![],
        recurring: vec![],
        settlements: vec![],
        accounts: vec![
            Account::new(a, AccountName::new("a").expect("name"), AccountKind::Cash),
            Account::new(b, AccountName::new("b").expect("name"), AccountKind::Bank),
        ],
        entries: vec![
            record(
                1,
                "2026-01-01",
                EntryKind::Opening {
                    account: a,
                    balance: "1000".parse().expect("money"),
                },
            ),
            record(
                2,
                "2026-01-01",
                EntryKind::Transfer {
                    from: a,
                    to: b,
                    amount: positive("100"),
                },
            ),
            record(
                3,
                "2025-12-31",
                EntryKind::Income {
                    account: a,
                    amount: positive("400"),
                    category: Category::OtherIncome,
                },
            ),
            record(
                4,
                "2026-01-01",
                EntryKind::Expense {
                    account: a,
                    amount: positive("30"),
                    category: Category::Food,
                },
            ),
            record(
                5,
                "2026-01-02",
                EntryKind::Income {
                    account: a,
                    amount: positive("999"),
                    category: Category::OtherIncome,
                },
            ),
            record(
                6,
                "2025-07-31",
                EntryKind::Expense {
                    account: a,
                    amount: positive("999"),
                    category: Category::Food,
                },
            ),
        ],
    };
    let today = "2026-01-01".parse().expect("date");
    let months =
        monthly_cashflow(&dashboard(state.clone(), today).expect("dashboard")).expect("months");
    assert_eq!(
        months.iter().map(|m| (m.year, m.month)).collect::<Vec<_>>(),
        [
            (2025, 8),
            (2025, 9),
            (2025, 10),
            (2025, 11),
            (2025, 12),
            (2026, 1)
        ]
    );
    assert_eq!(months[4].income.to_string(), "400.00");
    assert_eq!(months[5].expenses.to_string(), "30.00");
    assert_eq!(months[5].income, Money::ZERO);
    assert_eq!(months[0].health(), FlowHealth::NoData);
    state.entries.push(
        JournalEntry::reverse(
            "00000000-0000-0000-0000-000000000007".parse().expect("id"),
            &state.entries[2],
            Note::new("").expect("note"),
        )
        .expect("reverse"),
    );
    let months = monthly_cashflow(&dashboard(state, today).expect("dashboard")).expect("months");
    assert_eq!(months[4].income, Money::ZERO);
    assert_eq!(months[5].expenses.to_string(), "30.00");
}
