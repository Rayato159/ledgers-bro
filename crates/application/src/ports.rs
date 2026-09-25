use crate::{AccountDeletion, AppError, StorageError};
use ledger_domain::{Account, EntryDate, EntryId, JournalEntry, SubmissionId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerState {
    pub currency: ledger_domain::Currency,
    pub currency_locked: bool,
    pub thai_tax_enabled: bool,
    pub receivables: Vec<ledger_domain::Receivable>,
    pub accounts: Vec<Account>,
    pub entries: Vec<JournalEntry>,
    pub recurring: Vec<ledger_domain::RecurringExpense>,
    pub settlements: Vec<crate::RecurringSettlement>,
}

impl Default for LedgerState {
    fn default() -> Self {
        Self {
            currency: ledger_domain::Currency::Thb,
            currency_locked: false,
            thai_tax_enabled: true,
            receivables: Vec::new(),
            accounts: Vec::new(),
            entries: Vec::new(),
            recurring: Vec::new(),
            settlements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    Saved(EntryId),
    AlreadySaved(EntryId),
}

/// Each mutation is atomic and must recheck invariants against current data.
/// A snapshot is consistent across both collections. No partially saved postings.
pub trait LedgerRepository {
    fn set_credit_cycle(
        &mut self,
        expected: &Account,
        cycle: ledger_domain::CreditCardCycle,
    ) -> Result<(), StorageError>;
    fn preferences(&mut self) -> Result<crate::UserPreferences, StorageError>;
    fn set_preferences(&mut self, preferences: crate::UserPreferences) -> Result<(), StorageError>;
    fn set_thai_tax_enabled(&mut self, enabled: bool) -> Result<(), StorageError>;
    fn set_currency(&mut self, currency: ledger_domain::Currency) -> Result<(), StorageError>;
    fn commit_prompt(&mut self, plan: &crate::PromptPlan) -> Result<(), StorageError>;
    fn create_receivable(
        &mut self,
        prepared: &crate::PreparedReceivable,
    ) -> Result<CommitOutcome, StorageError>;
    fn set_recurring_installments(
        &mut self,
        expected: &ledger_domain::RecurringExpense,
        count: Option<u32>,
    ) -> Result<(), StorageError>;
    fn add_recurring(
        &mut self,
        schedule: &ledger_domain::RecurringExpense,
    ) -> Result<(), StorageError>;
    fn stop_recurring(
        &mut self,
        expected: &ledger_domain::RecurringExpense,
        month: ledger_domain::Month,
    ) -> Result<(), StorageError>;
    fn pay_recurring(
        &mut self,
        prepared: &crate::PreparedRecurringPayment,
    ) -> Result<CommitOutcome, StorageError>;
    fn link_recurring(
        &mut self,
        expected: &ledger_domain::RecurringExpense,
        month: ledger_domain::Month,
        entry: EntryId,
    ) -> Result<(), StorageError>;
    /// All entries save atomically, including validation and idempotent retries.
    fn commit_batch(
        &mut self,
        entries: &[crate::PreparedEntry],
    ) -> Result<Vec<CommitOutcome>, StorageError>;
    fn snapshot(&mut self) -> Result<LedgerState, StorageError>;
    /// Validate the reviewed state and remove the account plus whole journal
    /// entries atomically. A stale plan must fail without deleting anything.
    fn delete_account(&mut self, deletion: &AccountDeletion) -> Result<(), StorageError>;
    fn create_account(
        &mut self,
        account: &Account,
        opening: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError>;
    fn commit(
        &mut self,
        entry: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError>;
}

pub trait Clock {
    fn today(&self) -> Result<EntryDate, AppError>;
}
pub trait IdSource {
    fn receivable_id(&self) -> Result<ledger_domain::ReceivableId, AppError>;
    fn recurring_id(&self) -> Result<ledger_domain::RecurringId, AppError>;
    fn account_id(&self) -> Result<ledger_domain::AccountId, AppError>;
    fn entry_id(&self) -> Result<EntryId, AppError>;
    fn submission_id(&self) -> Result<SubmissionId, AppError>;
}

impl<T: IdSource + ?Sized> IdSource for &T {
    fn receivable_id(&self) -> Result<ledger_domain::ReceivableId, AppError> {
        (**self).receivable_id()
    }
    fn recurring_id(&self) -> Result<ledger_domain::RecurringId, AppError> {
        (**self).recurring_id()
    }
    fn account_id(&self) -> Result<ledger_domain::AccountId, AppError> {
        (**self).account_id()
    }
    fn entry_id(&self) -> Result<EntryId, AppError> {
        (**self).entry_id()
    }
    fn submission_id(&self) -> Result<SubmissionId, AppError> {
        (**self).submission_id()
    }
}

/// Called against the locked, current state by every durable repository adapter.
pub fn validate_append(state: &LedgerState, entry: &JournalEntry) -> Result<(), StorageError> {
    use ledger_domain::{DomainError, EntryKind};
    entry.validate_accounts(&state.accounts)?;
    if entry.income_tax().is_some()
        && (state.currency != ledger_domain::Currency::Thb || !state.thai_tax_enabled)
    {
        return Err(ledger_domain::DomainError::InvalidIncomeTax.into());
    }
    if state
        .entries
        .iter()
        .any(|existing| existing.id() == entry.id())
    {
        return Err(StorageError::SubmissionConflict);
    }
    if let EntryKind::Reversal { original } = entry.kind() {
        if state
            .entries
            .iter()
            .any(|e| matches!(e.kind(), EntryKind::Reversal { original: id } if id == original))
        {
            return Err(DomainError::InvalidReversal.into());
        }
        let original = state
            .entries
            .iter()
            .find(|e| e.id() == *original)
            .ok_or(DomainError::InvalidReversal)?;
        let expected = JournalEntry::reverse(entry.id(), original, entry.note().clone())?;
        if &expected != entry {
            return Err(StorageError::Corrupt);
        }
    }
    let mut prospective = state.clone();
    prospective.entries.push(entry.clone());
    crate::validate_receivables(&prospective)?;
    let latest = prospective
        .entries
        .iter()
        .map(JournalEntry::date)
        .max()
        .ok_or(StorageError::Corrupt)?;
    crate::dashboard(prospective, latest)?;
    Ok(())
}
