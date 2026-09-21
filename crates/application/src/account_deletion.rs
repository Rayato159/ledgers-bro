use crate::{LedgerState, StorageError, dashboard};
use ledger_domain::{
    Account, AccountId, DomainError, EntryDate, EntryId, EntryKind, Money, PostingTarget,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionBalanceChange {
    pub account: Account,
    pub before: Money,
    pub after: Money,
}

/// A reviewable deletion plan, bound to the exact ledger that was reviewed.
/// Repositories must validate it again under their write lock before deleting.
/// Whole journal aggregates are removed, including both sides of transfers and
/// their reversals; unrelated entries on the other accounts are retained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDeletion {
    account: Account,
    expected: LedgerState,
    entry_ids: Vec<EntryId>,
    transaction_count: usize,
    transfer_count: usize,
    reversal_count: usize,
    affected_accounts: Vec<DeletionBalanceChange>,
}

impl AccountDeletion {
    pub fn prepare(
        state: LedgerState,
        account_id: AccountId,
        today: EntryDate,
    ) -> Result<Self, StorageError> {
        let account = state
            .accounts
            .iter()
            .find(|a| a.id() == account_id)
            .ok_or(DomainError::AccountUnavailable)?
            .clone();
        let touches_account = |entry: &&ledger_domain::JournalEntry| {
            entry
                .postings()
                .iter()
                .any(|p| p.target() == PostingTarget::Account(account_id))
        };
        let removed: Vec<_> = state.entries.iter().filter(touches_account).collect();
        let entry_ids = removed.iter().rev().map(|entry| entry.id()).collect();
        let transaction_count = removed
            .iter()
            .filter(|entry| {
                matches!(
                    entry.kind(),
                    EntryKind::Income { .. }
                        | EntryKind::Expense { .. }
                        | EntryKind::Transfer { .. }
                )
            })
            .count();
        let transfer_count = removed
            .iter()
            .filter(|entry| matches!(entry.kind(), EntryKind::Transfer { .. }))
            .count();
        let reversal_count = removed
            .iter()
            .filter(|entry| matches!(entry.kind(), EntryKind::Reversal { .. }))
            .count();
        let related_ids: std::collections::BTreeSet<_> = removed
            .iter()
            .flat_map(|entry| entry.postings())
            .filter_map(|posting| match posting.target() {
                PostingTarget::Account(id) if id != account_id => Some(id),
                _ => None,
            })
            .collect();
        let remaining = LedgerState {
            accounts: state
                .accounts
                .iter()
                .filter(|a| a.id() != account_id)
                .cloned()
                .collect(),
            entries: state
                .entries
                .iter()
                .filter(|entry| !touches_account(entry))
                .cloned()
                .collect(),
        };
        // Validate the resulting balances too: removing an offsetting transfer
        // can overflow a remaining account even when the old balance was valid.
        let as_of = state
            .entries
            .iter()
            .map(|e| e.date())
            .max()
            .map_or(today, |date| date.max(today));
        let before = dashboard(state.clone(), as_of)?;
        let after = dashboard(remaining, as_of)?;
        let affected_accounts = after
            .accounts
            .into_iter()
            .filter(|a| related_ids.contains(&a.account.id()))
            .map(|a| {
                let previous = before
                    .accounts
                    .iter()
                    .find(|b| b.account.id() == a.account.id())
                    .ok_or(StorageError::Corrupt)?;
                Ok(DeletionBalanceChange {
                    account: a.account,
                    before: previous.balance,
                    after: a.balance,
                })
            })
            .collect::<Result<_, StorageError>>()?;
        Ok(Self {
            account,
            expected: state,
            entry_ids,
            transaction_count,
            transfer_count,
            reversal_count,
            affected_accounts,
        })
    }

    pub fn account(&self) -> &Account {
        &self.account
    }
    pub fn transaction_count(&self) -> usize {
        self.transaction_count
    }
    pub fn transfer_count(&self) -> usize {
        self.transfer_count
    }
    pub fn reversal_count(&self) -> usize {
        self.reversal_count
    }
    pub fn affected_accounts(&self) -> &[DeletionBalanceChange] {
        &self.affected_accounts
    }
    /// Reverse recording order ensures reversals are deleted before originals.
    pub fn entry_ids(&self) -> &[EntryId] {
        &self.entry_ids
    }

    pub fn validate_current(&self, current: &LedgerState) -> Result<(), StorageError> {
        if current != &self.expected {
            return Err(StorageError::DeletionChanged);
        }
        Ok(())
    }
}
