use crate::{AccountDeletion, AppError, StorageError};
use ledger_domain::{Account, EntryDate, EntryId, JournalEntry, SubmissionId};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerState {
    pub accounts: Vec<Account>,
    pub entries: Vec<JournalEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitOutcome {
    Saved(EntryId),
    AlreadySaved(EntryId),
}

/// Each mutation is atomic and must recheck invariants against current data.
/// A snapshot is consistent across both collections. No partially saved postings.
pub trait LedgerRepository {
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
    fn account_id(&self) -> Result<ledger_domain::AccountId, AppError>;
    fn entry_id(&self) -> Result<EntryId, AppError>;
    fn submission_id(&self) -> Result<SubmissionId, AppError>;
}

/// Called against the locked, current state by every durable repository adapter.
pub fn validate_append(state: &LedgerState, entry: &JournalEntry) -> Result<(), StorageError> {
    use ledger_domain::{DomainError, EntryKind};
    entry.validate_accounts(&state.accounts)?;
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
    let latest = prospective
        .entries
        .iter()
        .map(JournalEntry::date)
        .max()
        .ok_or(StorageError::Corrupt)?;
    crate::dashboard(prospective, latest)?;
    Ok(())
}
