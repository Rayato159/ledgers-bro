use crate::{Dashboard, LedgerState, StorageError};
use ledger_domain::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditBill {
    /// None means an opening balance or a legacy card without supplied terms.
    pub dates: Option<CreditStatementDates>,
    pub charged: Money,
    pub paid: Money,
    pub outstanding: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditCardSummary {
    pub account: Account,
    pub bills: Vec<CreditBill>,
    pub outstanding: Money,
    pub overdue: Money,
    pub prepaid: Money,
}

/// Derive obligations from the journal, never from a second debt ledger.
/// Credits settle oldest debits first. Reversed/future entries do not contribute.
/// Dates are estimates from user-entered posting dates, not imported bank statements.
pub fn credit_cards(view: &Dashboard) -> Result<Vec<CreditCardSummary>, DomainError> {
    let mut result = Vec::new();
    for balance in view
        .accounts
        .iter()
        .filter(|a| a.account.kind() == AccountKind::CreditCard)
    {
        let account = &balance.account;
        let mut entries: Vec<_> = view
            .entries
            .iter()
            .filter(|e| {
                e.date() <= view.today
                    && !view.reversed.contains(&e.id())
                    && !matches!(e.kind(), EntryKind::Reversal { .. })
            })
            .collect();
        // The dashboard is newest insertion first; stable sort retains the
        // journal insertion order for entries with the same effective date.
        entries.reverse();
        entries.sort_by_key(|e| e.date());
        let mut credits = 0_i128;
        let mut charges = Vec::new();
        for entry in entries {
            for posting in entry
                .postings()
                .iter()
                .filter(|p| p.target() == PostingTarget::Account(account.id()))
            {
                let minor = i128::from(posting.amount().minor());
                if minor >= 0 {
                    credits += minor;
                } else {
                    let dates = if matches!(entry.kind(), EntryKind::Opening { .. }) {
                        None
                    } else {
                        account
                            .credit_cycle()
                            .map(|c| c.statement_for(entry.date()))
                            .transpose()?
                    };
                    charges.push((dates, -minor));
                }
            }
        }
        let mut bills: BTreeMap<Option<CreditStatementDates>, (i128, i128)> = BTreeMap::new();
        for (dates, charged) in charges {
            let paid = charged.min(credits);
            credits -= paid;
            let bill = bills.entry(dates).or_default();
            bill.0 += charged;
            bill.1 += paid;
        }
        let mut outstanding = Money::ZERO;
        let mut overdue = Money::ZERO;
        let bills = bills
            .into_iter()
            .map(|(dates, (charged, paid))| {
                let remaining = money(charged - paid)?;
                outstanding = outstanding.checked_add(remaining)?;
                if dates.is_some_and(|d| d.due < view.today) {
                    overdue = overdue.checked_add(remaining)?;
                }
                Ok(CreditBill {
                    dates,
                    charged: money(charged)?,
                    paid: money(paid)?,
                    outstanding: remaining,
                })
            })
            .collect::<Result<Vec<_>, DomainError>>()?;
        // Fail visibly if a future journal kind is not correctly represented.
        if outstanding.minor() != (-balance.balance.minor()).max(0) {
            return Err(DomainError::UnbalancedJournal);
        }
        result.push(CreditCardSummary {
            account: account.clone(),
            bills,
            outstanding,
            overdue,
            prepaid: money(credits)?,
        });
    }
    Ok(result)
}

/// Actual expense cash payments plus card settlements this calendar month.
/// Buying on credit is an expense but is not yet a cash payment. Transfers
/// between assets, loan principal advances, and opening balances are excluded.
pub fn paid_out_this_month(view: &Dashboard) -> Result<Money, DomainError> {
    let is_credit = |id| {
        view.accounts
            .iter()
            .any(|a| a.account.id() == id && a.account.kind() == AccountKind::CreditCard)
    };
    let mut total = 0_i128;
    for entry in &view.entries {
        if entry.date() > view.today
            || entry.date().month_key() != view.today.month_key()
            || view.reversed.contains(&entry.id())
        {
            continue;
        }
        match entry.kind() {
            EntryKind::Expense {
                account, amount, ..
            } if !is_credit(*account) => total += i128::from(amount.money().minor()),
            EntryKind::Transfer { from, to, amount } if !is_credit(*from) && is_credit(*to) => {
                total += i128::from(amount.money().minor())
            }
            _ => {}
        }
    }
    money(total)
}

/// Legacy cards can acquire missing terms exactly once. Existing statement
/// terms are immutable so a settings edit cannot silently move historical bills.
pub fn configure_credit_cycle(
    state: &LedgerState,
    expected: &Account,
    cycle: CreditCardCycle,
) -> Result<Account, StorageError> {
    let account = state
        .accounts
        .iter()
        .find(|a| a.id() == expected.id())
        .ok_or(DomainError::AccountUnavailable)?;
    if account != expected || account.credit_cycle().is_some() || account.is_archived() {
        return Err(StorageError::CreditCycleChanged);
    }
    Ok(account.clone().with_credit_cycle(cycle)?)
}

fn money(minor: i128) -> Result<Money, DomainError> {
    Money::from_minor(i64::try_from(minor).map_err(|_| DomainError::MoneyOverflow)?)
}
