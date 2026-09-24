use crate::wire::StoredKind;
use ledger_application::{
    AccountDeletion, CommitOutcome, LedgerRepository, LedgerState, StorageError, validate_append,
};
use ledger_domain::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};

const APPLICATION_ID: i64 = 1_279_414_863;
const SCHEMA_VERSION: i64 = 1;

pub struct SqliteLedger {
    connection: Connection,
}
impl SqliteLedger {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let connection = Connection::open(path).map_err(database_error)?;
        Self::initialize(connection)
    }
    pub fn in_memory() -> Result<Self, StorageError> {
        Self::initialize(Connection::open_in_memory().map_err(database_error)?)
    }
    fn initialize(mut connection: Connection) -> Result<Self, StorageError> {
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(database_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")
            .map_err(database_error)?;
        // The write lock also serializes two app instances creating a fresh database.
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let version: i64 = tx
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(database_error)?;
        let app: i64 = tx
            .pragma_query_value(None, "application_id", |row| row.get(0))
            .map_err(database_error)?;
        if version > SCHEMA_VERSION {
            return Err(StorageError::NewerDatabase);
        }
        if version == 0 {
            let tables: i64 = tx.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0)).map_err(database_error)?;
            if tables != 0 || (app != 0 && app != APPLICATION_ID) {
                return Err(StorageError::Corrupt);
            }
            tx.execute_batch(include_str!("../migrations/001_ledger.sql"))
                .map_err(database_error)?;
        } else if app != APPLICATION_ID {
            return Err(StorageError::Corrupt);
        }
        tx.commit().map_err(database_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(database_error)?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .map_err(database_error)?;
        Ok(Self { connection })
    }
}

impl LedgerRepository for SqliteLedger {
    fn commit_batch(
        &mut self,
        entries: &[ledger_application::PreparedEntry],
    ) -> Result<Vec<CommitOutcome>, StorageError> {
        if entries.is_empty() || entries.len() > ledger_application::MAX_BATCH_ENTRIES {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let mut state = read_state(&tx)?;
        let mut outcomes = Vec::new();
        let mut submissions = BTreeSet::new();
        let mut ids = BTreeSet::new();
        for prepared in entries {
            if !submissions.insert(prepared.submission)
                || !ids.insert(prepared.entry.id())
                || matches!(
                    prepared.entry.kind(),
                    EntryKind::Opening { .. } | EntryKind::Reversal { .. }
                )
            {
                return Err(StorageError::SubmissionConflict);
            }
            if let Some(outcome) = existing_submission(&tx, &prepared.entry, prepared.submission)? {
                outcomes.push(outcome);
            } else {
                validate_append(&state, &prepared.entry)?;
                insert_entry(&tx, &prepared.entry, prepared.submission)?;
                state.entries.push(prepared.entry.clone());
                outcomes.push(CommitOutcome::Saved(prepared.entry.id()));
            }
        }
        tx.commit().map_err(database_error)?;
        Ok(outcomes)
    }
    fn delete_account(&mut self, deletion: &AccountDeletion) -> Result<(), StorageError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        deletion.validate_current(&read_state(&tx)?)?;
        for id in deletion.entry_ids() {
            tx.execute("DELETE FROM postings WHERE entry_id=?1", [id.to_string()])
                .map_err(database_error)?;
            let deleted = tx
                .execute("DELETE FROM journal_entries WHERE id=?1", [id.to_string()])
                .map_err(database_error)?;
            if deleted != 1 {
                return Err(StorageError::Corrupt);
            }
        }
        let deleted = tx
            .execute(
                "DELETE FROM accounts WHERE id=?1",
                [deletion.account().id().to_string()],
            )
            .map_err(database_error)?;
        if deleted != 1 {
            return Err(StorageError::Corrupt);
        }
        tx.commit().map_err(database_error)?;
        Ok(())
    }
    fn snapshot(&mut self) -> Result<LedgerState, StorageError> {
        let tx = self.connection.transaction().map_err(database_error)?;
        let state = read_state(&tx)?;
        tx.commit().map_err(database_error)?;
        Ok(state)
    }
    fn create_account(
        &mut self,
        account: &Account,
        opening: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError> {
        if !matches!(opening.kind(), EntryKind::Opening { account: id, .. } if *id == account.id())
        {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if let Some(outcome) = existing_submission(&tx, opening, submission)? {
            return Ok(outcome);
        }
        let mut state = read_state(&tx)?;
        account.ensure_can_add(&state.accounts)?;
        state.accounts.push(account.clone());
        validate_append(&state, opening)?;
        tx.execute(
            "INSERT INTO accounts(id,name,name_key,kind,archived) VALUES(?1,?2,?3,?4,?5)",
            params![
                account.id().to_string(),
                account.name().as_str(),
                account.name().key(),
                account.kind().code(),
                account.is_archived()
            ],
        )
        .map_err(database_error)?;
        insert_entry(&tx, opening, submission)?;
        tx.commit().map_err(database_error)?;
        Ok(CommitOutcome::Saved(opening.id()))
    }
    fn commit(
        &mut self,
        entry: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError> {
        if matches!(entry.kind(), EntryKind::Opening { .. }) {
            return Err(StorageError::Corrupt);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(database_error)?;
        if let Some(outcome) = existing_submission(&tx, entry, submission)? {
            return Ok(outcome);
        }
        validate_append(&read_state(&tx)?, entry)?;
        insert_entry(&tx, entry, submission)?;
        tx.commit().map_err(database_error)?;
        Ok(CommitOutcome::Saved(entry.id()))
    }
}

fn existing_submission(
    connection: &Connection,
    entry: &JournalEntry,
    submission: SubmissionId,
) -> Result<Option<CommitOutcome>, StorageError> {
    let saved = connection
        .query_row(
            "SELECT id,effective_date,note,payload FROM journal_entries WHERE submission_id=?1",
            [submission.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(database_error)?;
    if let Some((id, date, note, payload)) = saved {
        if date != entry.date().to_string()
            || note != entry.note().as_str()
            || payload != StoredKind::encode(entry)?
        {
            return Err(StorageError::SubmissionConflict);
        }
        return Ok(Some(CommitOutcome::AlreadySaved(id.parse()?)));
    }
    Ok(None)
}

fn insert_entry(
    connection: &Connection,
    entry: &JournalEntry,
    submission: SubmissionId,
) -> Result<(), StorageError> {
    let reverses = match entry.kind() {
        EntryKind::Reversal { original } => Some(original.to_string()),
        _ => None,
    };
    connection.execute("INSERT INTO journal_entries(id,submission_id,effective_date,note,payload,reverses) VALUES(?1,?2,?3,?4,?5,?6)", params![entry.id().to_string(), submission.to_string(), entry.date().to_string(), entry.note().as_str(), StoredKind::encode(entry)?, reverses]).map_err(database_error)?;
    for (ordinal, posting) in entry.postings().iter().enumerate() {
        let (account, book) = target_columns(posting.target());
        connection.execute("INSERT INTO postings(entry_id,ordinal,account_id,system_book,amount_minor) VALUES(?1,?2,?3,?4,?5)", params![entry.id().to_string(), ordinal as i64, account, book, posting.amount().minor()]).map_err(database_error)?;
    }
    Ok(())
}

fn target_columns(target: PostingTarget) -> (Option<String>, Option<&'static str>) {
    match target {
        PostingTarget::Account(id) => (Some(id.to_string()), None),
        PostingTarget::System(book) => (
            None,
            Some(match book {
                SystemBook::Equity => "equity",
                SystemBook::Income => "income",
                SystemBook::Expense => "expense",
            }),
        ),
    }
}

fn read_state(connection: &Connection) -> Result<LedgerState, StorageError> {
    let mut accounts_query = connection
        .prepare("SELECT id,name,name_key,kind,archived FROM accounts ORDER BY name_key,id")
        .map_err(database_error)?;
    let mut accounts_rows = accounts_query.query([]).map_err(database_error)?;
    let mut accounts = Vec::new();
    while let Some(row) = accounts_rows.next().map_err(database_error)? {
        let id: String = row.get(0).map_err(database_error)?;
        let name = AccountName::new(&row.get::<_, String>(1).map_err(database_error)?)
            .map_err(|_| StorageError::Corrupt)?;
        let key: String = row.get(2).map_err(database_error)?;
        if key != name.key() {
            return Err(StorageError::Corrupt);
        }
        let kind: String = row.get(3).map_err(database_error)?;
        accounts.push(Account::restore(
            id.parse().map_err(|_| StorageError::Corrupt)?,
            name,
            AccountKind::from_code(&kind).map_err(|_| StorageError::Corrupt)?,
            row.get(4).map_err(database_error)?,
        ));
    }
    if accounts.len() > MAX_ACCOUNTS {
        return Err(StorageError::Corrupt);
    }
    let mut query = connection
        .prepare(
            "SELECT id,effective_date,note,payload,reverses FROM journal_entries ORDER BY sequence",
        )
        .map_err(database_error)?;
    let mut rows = query.query([]).map_err(database_error)?;
    let mut lookup = BTreeMap::new();
    let mut reversed = BTreeSet::new();
    let mut entries = Vec::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let id: String = row.get(0).map_err(database_error)?;
        let date: String = row.get(1).map_err(database_error)?;
        let note: String = row.get(2).map_err(database_error)?;
        let payload: String = row.get(3).map_err(database_error)?;
        let reverses: Option<String> = row.get(4).map_err(database_error)?;
        let stored: StoredKind =
            serde_json::from_str(&payload).map_err(|_| StorageError::Corrupt)?;
        let entry = stored
            .restore(
                id.parse().map_err(|_| StorageError::Corrupt)?,
                date.parse().map_err(|_| StorageError::Corrupt)?,
                Note::new(&note).map_err(|_| StorageError::Corrupt)?,
                &lookup,
            )
            .map_err(|_| StorageError::Corrupt)?;
        match entry.kind() {
            EntryKind::Reversal { original }
                if reverses.as_deref() == Some(original.to_string().as_str())
                    && reversed.insert(*original) => {}
            EntryKind::Reversal { .. } => return Err(StorageError::Corrupt),
            _ if reverses.is_some() => return Err(StorageError::Corrupt),
            _ => {}
        }
        let mut p_query = connection.prepare("SELECT account_id,system_book,amount_minor FROM postings WHERE entry_id=?1 ORDER BY ordinal").map_err(database_error)?;
        let persisted = p_query
            .query_map([&id], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        if persisted.len() != entry.postings().len() {
            return Err(StorageError::Corrupt);
        }
        for (actual, expected) in persisted.iter().zip(entry.postings()) {
            let (account, book) = target_columns(expected.target());
            if actual.0 != account
                || actual.1.as_deref() != book
                || actual.2 != expected.amount().minor()
            {
                return Err(StorageError::Corrupt);
            }
            if let PostingTarget::Account(id) = expected.target()
                && !accounts.iter().any(|a| a.id() == id)
            {
                return Err(StorageError::Corrupt);
            }
        }
        lookup.insert(entry.id(), entry.clone());
        entries.push(entry);
    }
    Ok(LedgerState { accounts, entries })
}

fn database_error(error: rusqlite::Error) -> StorageError {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseCorrupt | rusqlite::ErrorCode::NotADatabase) => {
            StorageError::Corrupt
        }
        _ => StorageError::Unavailable,
    }
}
