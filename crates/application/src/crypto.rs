use crate::{AppError, Dashboard, LedgerState, StorageError};
use ledger_domain::*;

pub trait CryptoPriceSource: Send + Sync {
    fn fetch(&self, now: i64) -> Result<CryptoPrices, AppError>;
}

pub fn configure_crypto_holdings(
    state: &LedgerState,
    expected: &Account,
    holdings: CryptoHoldings,
) -> Result<Account, StorageError> {
    if state.currency != Currency::Thb {
        return Err(DomainError::InvalidCurrency.into());
    }
    if !state.accounts.contains(expected) || expected.is_archived() {
        return Err(StorageError::CryptoHoldingsChanged);
    }
    if expected.crypto_holdings().is_none()
        && state
            .recurring
            .iter()
            .any(|s| s.account() == Some(expected.id()) && s.stopped_from().is_none())
    {
        return Err(StorageError::CryptoRecurringAccount);
    }
    Ok(expected.clone().with_crypto_holdings(holdings)?)
}

/// Enforce native-unit accounts at preview AND inside the repository write lock.
/// Old cash entries and their reversals remain readable after explicit conversion.
pub fn validate_crypto_cash_entry(
    state: &LedgerState,
    entry: &JournalEntry,
) -> Result<(), DomainError> {
    if matches!(entry.kind(), EntryKind::Reversal { .. }) {
        return Ok(());
    }
    for account in state
        .accounts
        .iter()
        .filter(|a| a.crypto_holdings().is_some())
    {
        if entry
            .postings()
            .iter()
            .any(|p| p.target() == PostingTarget::Account(account.id()))
            && !matches!(entry.kind(), EntryKind::Opening { balance, .. } if *balance == Money::ZERO)
        {
            return Err(DomainError::CryptoCashEntry);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CryptoValuation {
    pub assets: Money,
    pub liabilities: Money,
    pub net_worth: Money,
    pub crypto_value: Money,
    pub unpriced_portfolios: usize,
    pub portfolios: usize,
}

/// Market values replace a converted account's book balance in the live overview;
/// quotes never produce postings, income, expenses, or tax events.
pub fn crypto_valuation(
    view: &Dashboard,
    prices: Option<CryptoPrices>,
    now: i64,
) -> Result<CryptoValuation, DomainError> {
    let mut result = CryptoValuation {
        assets: view.assets,
        liabilities: view.liabilities,
        net_worth: view.net_worth,
        crypto_value: Money::ZERO,
        unpriced_portfolios: 0,
        portfolios: 0,
    };
    let prices = prices.and_then(|p| p.validate_at(now).ok());
    for item in &view.accounts {
        let Some(holdings) = item.account.crypto_holdings() else {
            continue;
        };
        if view.currency != Currency::Thb {
            return Err(DomainError::InvalidCurrency);
        }
        result.portfolios += 1;
        if item.balance >= Money::ZERO {
            result.assets = result.assets.checked_sub(item.balance)?;
        } else {
            result.liabilities = result.liabilities.checked_add(item.balance)?;
        }
        let value = if holdings.is_empty() {
            Some(Money::ZERO)
        } else {
            prices.map(|p| p.value(holdings)).transpose()?
        };
        if let Some(value) = value {
            result.assets = result.assets.checked_add(value)?;
            result.crypto_value = result.crypto_value.checked_add(value)?;
        } else {
            result.unpriced_portfolios += 1;
        }
    }
    result.net_worth = result.assets.checked_sub(result.liabilities)?;
    Ok(result)
}
