#![allow(clippy::expect_used)]
use ledger_domain::*;
use uuid::Uuid;

fn account_id(n: u128) -> AccountId {
    AccountId::from_uuid(Uuid::from_u128(n)).expect("fixture id")
}
fn entry_id(n: u128) -> EntryId {
    EntryId::from_uuid(Uuid::from_u128(n)).expect("fixture id")
}
fn amount(text: &str) -> PositiveMoney {
    PositiveMoney::new(text.parse().expect("fixture money")).expect("positive fixture")
}
fn date() -> EntryDate {
    "2026-09-20".parse().expect("fixture date")
}

#[test]
fn money_is_exact_and_rejects_ambiguous_notation() {
    assert_eq!("0.1".parse::<Money>().expect("decimal").minor(), 10);
    assert_eq!("-0.01".parse::<Money>().expect("decimal").minor(), -1);
    assert_eq!(
        "100.09".parse::<Money>().expect("decimal").to_string(),
        "100.09"
    );
    for invalid in [
        "",
        " ",
        "1,000",
        "1,2",
        "12O",
        "NaN",
        "1e3",
        "+5",
        ".5",
        "1.",
        "1.001",
        "1.2.3",
        "--1",
        "๑๒",
        "90000000000.01",
        "-92233720368547758.09",
    ] {
        assert!(invalid.parse::<Money>().is_err(), "accepted {invalid}");
    }
    assert!(PositiveMoney::new(Money::ZERO).is_err());
    assert!(Money::from_minor(i64::MIN).is_err());
    assert!(
        Money::from_minor(Money::MAX_MINOR)
            .expect("bound")
            .checked_add(Money::from_minor(1).expect("one"))
            .is_err()
    );
}

#[test]
fn cents_round_trip_without_float_loss() {
    for cents in (-100_000..100_000).step_by(13) {
        let value = Money::from_minor(cents).expect("bounded value");
        assert_eq!(value.to_string().parse::<Money>(), Ok(value));
        assert_eq!(value.checked_add(value.negated()), Ok(Money::ZERO));
    }
    for cents in [Money::MAX_MINOR, -Money::MAX_MINOR] {
        let value = Money::from_minor(cents).expect("bound");
        assert_eq!(value.to_string().parse::<Money>(), Ok(value));
    }
}

#[test]
fn identities_and_calendar_dates_are_validated() {
    assert!(AccountId::from_uuid(Uuid::nil()).is_err());
    assert!("not-an-id".parse::<EntryId>().is_err());
    assert!("2025-02-29".parse::<EntryDate>().is_err());
    assert_eq!(
        "2024-03-01"
            .parse::<EntryDate>()
            .expect("leap date")
            .previous_day()
            .expect("previous")
            .to_string(),
        "2024-02-29"
    );
    assert_eq!(
        "2026-01-01"
            .parse::<EntryDate>()
            .expect("new year")
            .previous_day()
            .expect("previous")
            .to_string(),
        "2025-12-31"
    );
    assert!("2026-9-1".parse::<EntryDate>().is_err());
}

#[test]
fn account_identity_is_independent_of_its_name() {
    let first = Account::new(
        account_id(1),
        AccountName::new("  My   Bank ").expect("name"),
        AccountKind::Bank,
    );
    let second = Account::new(
        account_id(2),
        AccountName::new("my bank").expect("name"),
        AccountKind::Bank,
    );
    assert_ne!(first.id(), second.id());
    assert_eq!(first.name().as_str(), "My Bank");
    assert_eq!(
        second.ensure_can_add(&[first]),
        Err(DomainError::DuplicateAccountName)
    );
    assert!(AccountName::new("name\nother").is_err());
    assert!(Note::new(&"a".repeat(Note::MAX_CHARS + 1)).is_err());
}

#[test]
fn account_limit_counts_archived_accounts() {
    let existing: Vec<_> = (1..=100)
        .map(|n| {
            Account::restore(
                account_id(n),
                AccountName::new(&format!("Account {n}")).expect("name"),
                AccountKind::Cash,
                true,
            )
        })
        .collect();
    let candidate = Account::new(
        account_id(101),
        AccountName::new("new").expect("name"),
        AccountKind::Cash,
    );
    assert_eq!(
        candidate.ensure_can_add(&existing),
        Err(DomainError::AccountLimit)
    );
}

#[test]
fn journal_factories_enforce_category_direction_and_distinct_transfers() {
    assert_eq!(
        JournalEntry::record(
            entry_id(1),
            date(),
            Note::default(),
            EntryKind::Transfer {
                from: account_id(1),
                to: account_id(1),
                amount: amount("1")
            }
        ),
        Err(DomainError::SameAccountTransfer)
    );
    assert_eq!(
        JournalEntry::record(
            entry_id(2),
            date(),
            Note::default(),
            EntryKind::Expense {
                account: account_id(1),
                amount: amount("10"),
                category: Category::Salary
            }
        ),
        Err(DomainError::InvalidCategory)
    );
    assert_eq!(
        JournalEntry::record(
            entry_id(3),
            date(),
            Note::default(),
            EntryKind::Income {
                account: account_id(1),
                amount: amount("10"),
                category: Category::Food
            }
        ),
        Err(DomainError::InvalidCategory)
    );
}

#[test]
fn every_posting_pair_is_balanced_and_reversal_is_exact() {
    for minor in [1, 10, 10_000, Money::MAX_MINOR] {
        let money =
            PositiveMoney::new(Money::from_minor(minor).expect("amount")).expect("positive");
        let original = JournalEntry::record(
            entry_id(1),
            date(),
            Note::default(),
            EntryKind::Expense {
                account: account_id(1),
                amount: money,
                category: Category::Food,
            },
        )
        .expect("entry");
        assert_eq!(
            original
                .postings()
                .iter()
                .map(|p| i128::from(p.amount().minor()))
                .sum::<i128>(),
            0
        );
        let reversed =
            JournalEntry::reverse(entry_id(2), &original, Note::default()).expect("reversal");
        assert_eq!(reversed.date(), original.date());
        for (a, b) in original.postings().iter().zip(reversed.postings()) {
            assert_eq!(a.target(), b.target());
            assert_eq!(a.amount().checked_add(b.amount()), Ok(Money::ZERO));
        }
        assert!(JournalEntry::reverse(entry_id(3), &reversed, Note::default()).is_err());
    }
}

#[test]
fn archived_or_missing_accounts_are_rejected() {
    let entry = JournalEntry::record(
        entry_id(1),
        date(),
        Note::default(),
        EntryKind::Expense {
            account: account_id(1),
            amount: amount("10"),
            category: Category::Food,
        },
    )
    .expect("entry");
    assert_eq!(
        entry.validate_accounts(&[]),
        Err(DomainError::AccountUnavailable)
    );
    let account = Account::restore(
        account_id(1),
        AccountName::new("cash").expect("name"),
        AccountKind::Cash,
        true,
    );
    assert_eq!(
        entry.validate_accounts(&[account]),
        Err(DomainError::AccountUnavailable)
    );
}
