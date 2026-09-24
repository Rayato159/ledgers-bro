use crate::LedgerState;
use ledger_domain::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBalance {
    pub account: Account,
    pub balance: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dashboard {
    pub currency: Currency,
    pub currency_locked: bool,
    pub thai_tax_enabled: bool,
    pub receivables: Vec<Receivable>,
    pub today: EntryDate,
    pub accounts: Vec<AccountBalance>,
    pub assets: Money,
    pub liabilities: Money,
    pub net_worth: Money,
    pub income: Money,
    pub expenses: Money,
    pub category_expenses: BTreeMap<Category, Money>,
    pub entries: Vec<JournalEntry>,
    pub reversed: BTreeSet<EntryId>,
    pub recurring: Vec<RecurringExpense>,
    pub settlements: Vec<crate::RecurringSettlement>,
}

/// Net worth uses signed ledger balances. A credit card's negative balance is debt,
/// while an overpayment is an asset. Opening balances and transfers are not income.
pub fn dashboard(state: LedgerState, today: EntryDate) -> Result<Dashboard, DomainError> {
    let mut balances: BTreeMap<AccountId, i128> =
        state.accounts.iter().map(|a| (a.id(), 0)).collect();
    let reversed = state
        .entries
        .iter()
        .filter_map(|e| match e.kind() {
            EntryKind::Reversal { original } => Some(*original),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut income: i128 = 0;
    let mut receivable_assets: i128 = 0;
    let mut expenses: i128 = 0;
    let mut categories: BTreeMap<Category, i128> = BTreeMap::new();
    for entry in &state.entries {
        if entry.date() > today {
            continue;
        }
        for posting in entry.postings() {
            if posting.target() == PostingTarget::System(SystemBook::Receivable) {
                receivable_assets += i128::from(posting.amount().minor());
            }
            if let PostingTarget::Account(id) = posting.target() {
                let balance = balances
                    .get_mut(&id)
                    .ok_or(DomainError::AccountUnavailable)?;
                *balance += i128::from(posting.amount().minor());
            }
        }
        if entry.date().month_key() == today.month_key() && !reversed.contains(&entry.id()) {
            match entry.kind() {
                EntryKind::Income { amount, .. } => income += i128::from(amount.money().minor()),
                EntryKind::Expense {
                    amount, category, ..
                } => {
                    let minor = i128::from(amount.money().minor());
                    expenses += minor;
                    *categories.entry(*category).or_default() += minor;
                }
                _ => {}
            }
        }
    }
    let mut assets = 0_i128;
    let mut liabilities = 0_i128;
    let mut accounts = Vec::with_capacity(state.accounts.len());
    for account in state.accounts {
        let balance = *balances
            .get(&account.id())
            .ok_or(DomainError::AccountUnavailable)?;
        if balance >= 0 {
            assets += balance;
        } else {
            liabilities -= balance;
        }
        accounts.push(AccountBalance {
            account,
            balance: checked_money(balance)?,
        });
    }
    assets += receivable_assets;
    Ok(Dashboard {
        currency: state.currency,
        currency_locked: state.currency_locked,
        thai_tax_enabled: state.thai_tax_enabled,
        receivables: state.receivables,
        today,
        accounts,
        assets: checked_money(assets)?,
        liabilities: checked_money(liabilities)?,
        net_worth: checked_money(assets - liabilities)?,
        income: checked_money(income)?,
        expenses: checked_money(expenses)?,
        category_expenses: categories
            .into_iter()
            .map(|(category, minor)| Ok((category, checked_money(minor)?)))
            .collect::<Result<_, DomainError>>()?,
        entries: state.entries.into_iter().rev().collect(),
        reversed,
        recurring: state.recurring,
        settlements: state.settlements,
    })
}

fn checked_money(value: i128) -> Result<Money, DomainError> {
    Money::from_minor(i64::try_from(value).map_err(|_| DomainError::MoneyOverflow)?)
}
