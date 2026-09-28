#![allow(clippy::expect_used)]
use ledger_application::{
    AccountBalance, LedgerState, account_valuation, crypto_valuation, dashboard,
};
use ledger_domain::*;

fn account(index: u32, kind: AccountKind, balance: &str) -> AccountBalance {
    AccountBalance {
        account: Account::new(
            format!("00000000-0000-0000-0000-{index:012}")
                .parse()
                .expect("id"),
            AccountName::new(&format!("Synthetic account {index}")).expect("name"),
            kind,
        ),
        balance: balance.parse().expect("balance"),
    }
}

#[test]
fn all_accounts_totals_include_debt_and_overpayments_without_receivables() {
    let mut view =
        dashboard(LedgerState::default(), "2026-09-28".parse().expect("date")).expect("view");
    view.accounts = vec![
        account(1, AccountKind::Cash, "100"),
        account(2, AccountKind::Bank, "900"),
        account(3, AccountKind::CreditCard, "-250"),
        account(4, AccountKind::CreditCard, "50"),
        account(5, AccountKind::Investment, "200"),
        account(6, AccountKind::Crypto, "75"),
    ];
    // Overview also includes receivable assets; account totals must not.
    view.assets = "6325".parse().expect("assets");
    view.liabilities = "250".parse().expect("debt");
    let totals =
        account_valuation(&view.accounts, Currency::Thb, None, 1_790_330_700).expect("totals");
    assert_eq!(totals.assets.to_string(), "1325.00");
    assert_eq!(totals.liabilities.to_string(), "250.00");
    assert_eq!(totals.net_worth.to_string(), "1075.00");
    assert_eq!(
        crypto_valuation(&view, None, 1_790_330_700)
            .expect("overview")
            .assets
            .to_string(),
        "6325.00"
    );
}

#[test]
fn native_crypto_replaces_book_value_and_missing_quotes_are_explicit() {
    let mut coin = account(1, AccountKind::Crypto, "999");
    coin.account = coin
        .account
        .with_crypto_holdings(CryptoHoldings::parse("0.5", "2").expect("holdings"))
        .expect("crypto");
    let accounts = [coin, account(2, AccountKind::Bank, "10")];
    let missing =
        account_valuation(&accounts, Currency::Thb, None, 1_790_330_700).expect("unpriced");
    assert_eq!(missing.net_worth.to_string(), "10.00");
    assert_eq!(missing.unpriced_portfolios, 1);
    let quote = |price| {
        CryptoQuote::new(ThbUnitPrice::parse(price).expect("price"), 1_790_330_700).expect("quote")
    };
    let priced = account_valuation(
        &accounts,
        Currency::Thb,
        Some(CryptoPrices {
            bitcoin: quote("100"),
            solana: quote("5"),
        }),
        1_790_330_700,
    )
    .expect("priced");
    assert_eq!(priced.net_worth.to_string(), "70.00");
    assert_eq!(priced.crypto_value.to_string(), "60.00");
    assert_eq!(priced.unpriced_portfolios, 0);
    assert_eq!(
        accounts[0].balance.to_string(),
        "999.00",
        "valuation must not rewrite the ledger"
    );
}

#[test]
fn no_accounts_has_zero_totals() {
    let totals = account_valuation(&[], Currency::Usd, None, 1_790_330_700).expect("empty");
    assert_eq!(totals.assets, Money::ZERO);
    assert_eq!(totals.liabilities, Money::ZERO);
    assert_eq!(totals.net_worth, Money::ZERO);
}
