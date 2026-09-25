//! Versioned logical backups. Never execute a schema, SQL, or filesystem path
//! supplied by an imported file. Restore into our trusted schema atomically.
use crate::{SqliteLedger, sqlite::read_state};
use ledger_application::{BackupSummary, LedgerRepository, MAX_BACKUP_BYTES, StorageError};
use rusqlite::{
    Connection, TransactionBehavior,
    types::{Value, ValueRef},
};
use serde::{Deserialize, Serialize};

const TABLES: &[&str] = &[
    "ledger_settings",
    "user_preferences",
    "accounts",
    "recurring_expenses",
    "receivables",
    "journal_entries",
    "postings",
    "recurring_settlements",
    "prompt_submissions",
    "crypto_holding_changes",
    "crypto_prices",
];

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Backup {
    format: String,
    version: u32,
    schema: u32,
    tables: Vec<Table>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Table {
    name: String,
    columns: Vec<String>,
    rows: Vec<Vec<Cell>>,
}
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", deny_unknown_fields)]
enum Cell {
    Null,
    Integer(i64),
    Text(String),
}
impl Cell {
    fn sql(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Integer(n) => Value::Integer(*n),
            Self::Text(s) => Value::Text(s.clone()),
        }
    }
}
fn invalid<T>(_: T) -> StorageError {
    StorageError::InvalidBackup
}
fn columns(connection: &Connection, table: &str) -> Result<Vec<String>, StorageError> {
    // table always comes from the compiled whitelist.
    connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(invalid)?
        .query_map([], |r| r.get(1))
        .map_err(invalid)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(invalid)
}
fn dump(connection: &Connection) -> Result<Backup, StorageError> {
    let mut tables = Vec::new();
    for &name in TABLES {
        let columns = columns(connection, name)?;
        let mut query = connection
            .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
            .map_err(invalid)?;
        let mut records = query.query([]).map_err(invalid)?;
        let mut rows = Vec::new();
        while let Some(record) = records.next().map_err(invalid)? {
            let mut row = Vec::new();
            for index in 0..columns.len() {
                row.push(match record.get_ref(index).map_err(invalid)? {
                    ValueRef::Null => Cell::Null,
                    ValueRef::Integer(n) => Cell::Integer(n),
                    ValueRef::Text(s) => {
                        Cell::Text(std::str::from_utf8(s).map_err(invalid)?.to_owned())
                    }
                    _ => return Err(StorageError::InvalidBackup),
                });
            }
            rows.push(row);
        }
        tables.push(Table {
            name: name.into(),
            columns,
            rows,
        });
    }
    Ok(Backup {
        format: "ledgers-bro-backup".into(),
        version: 1,
        schema: 10,
        tables,
    })
}
fn summary(connection: &Connection) -> Result<BackupSummary, StorageError> {
    let state = read_state(connection).map_err(invalid)?;
    // Read-model validation reconstructs every entry and compares its persisted
    // debit/credit postings. Also check totals and references before any restore.
    let latest = state
        .entries
        .iter()
        .map(|e| e.date())
        .max()
        .unwrap_or("2000-01-01".parse().map_err(invalid)?);
    ledger_application::dashboard(state.clone(), latest).map_err(invalid)?;
    if connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(invalid)?
        .query([])
        .map_err(invalid)?
        .next()
        .map_err(invalid)?
        .is_some()
    {
        return Err(StorageError::InvalidBackup);
    }
    Ok(BackupSummary {
        accounts: state.accounts.len(),
        transactions: state.entries.len(),
        plans: state.recurring.len(),
        receivables: state.receivables.len(),
        currency: state.currency,
    })
}
pub(crate) fn export(ledger: &mut SqliteLedger) -> Result<Vec<u8>, StorageError> {
    let tx = ledger.connection.transaction().map_err(invalid)?;
    summary(&tx)?;
    let bytes = serde_json::to_vec(&dump(&tx)?).map_err(invalid)?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(StorageError::InvalidBackup);
    }
    tx.commit().map_err(invalid)?;
    Ok(bytes)
}
fn decode(bytes: &[u8]) -> Result<Backup, StorageError> {
    if bytes.is_empty() || bytes.len() > MAX_BACKUP_BYTES {
        return Err(StorageError::InvalidBackup);
    }
    let backup: Backup = serde_json::from_slice(bytes).map_err(invalid)?;
    if backup.format != "ledgers-bro-backup"
        || backup.version != 1
        || backup.schema != 10
        || backup.tables.len() != TABLES.len()
    {
        return Err(StorageError::InvalidBackup);
    }
    Ok(backup)
}
fn empty(connection: &Connection) -> Result<(), StorageError> {
    for &table in TABLES {
        if matches!(
            table,
            "ledger_settings" | "user_preferences" | "crypto_prices"
        ) {
            continue;
        }
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .map_err(invalid)?;
        if count != 0 {
            return Err(StorageError::RestoreNeedsEmptyLedger);
        }
    }
    Ok(())
}
fn populate(connection: &Connection, backup: &Backup) -> Result<BackupSummary, StorageError> {
    empty(connection)?;
    connection
        .execute_batch(
            "DELETE FROM ledger_settings; DELETE FROM user_preferences; DELETE FROM crypto_prices;",
        )
        .map_err(invalid)?;
    for (table, &name) in backup.tables.iter().zip(TABLES) {
        let expected = columns(connection, name)?;
        if table.name != name || table.columns != expected || table.rows.len() > 200_000 {
            return Err(StorageError::InvalidBackup);
        }
        let placeholders = vec!["?"; expected.len()].join(",");
        let mut insert = connection
            .prepare(&format!("INSERT INTO {name} VALUES({placeholders})"))
            .map_err(invalid)?;
        for row in &table.rows {
            if row.len() != expected.len() {
                return Err(StorageError::InvalidBackup);
            }
            insert
                .execute(rusqlite::params_from_iter(row.iter().map(Cell::sql)))
                .map_err(invalid)?;
        }
    }
    let result = summary(connection)?;
    if &dump(connection)? != backup {
        return Err(StorageError::InvalidBackup);
    }
    Ok(result)
}
fn validated(bytes: &[u8]) -> Result<(Backup, BackupSummary), StorageError> {
    let backup = decode(bytes)?;
    let ledger = SqliteLedger::in_memory()?;
    let summary = populate(&ledger.connection, &backup)?;
    // Validate singleton configuration through its regular repository adapters.
    let mut ledger = ledger;
    ledger.preferences().map_err(invalid)?;
    ledger.crypto_prices().map_err(invalid)?;
    Ok((backup, summary))
}
pub(crate) fn preview(bytes: &[u8]) -> Result<BackupSummary, StorageError> {
    validated(bytes).map(|(_, summary)| summary)
}
pub(crate) fn restore(
    ledger: &mut SqliteLedger,
    bytes: &[u8],
) -> Result<BackupSummary, StorageError> {
    let (backup, expected) = validated(bytes)?;
    let tx = ledger
        .connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(invalid)?;
    let result = populate(&tx, &backup)?;
    if result != expected {
        return Err(StorageError::InvalidBackup);
    }
    tx.commit().map_err(invalid)?;
    Ok(result)
}
