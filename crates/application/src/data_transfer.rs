use crate::*;
use ledger_domain::*;
use std::{collections::BTreeSet, sync::Arc};

pub const MAX_IMPORT_ROWS: usize = 1000;
pub const MAX_CSV_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BACKUP_BYTES: usize = 32 * 1024 * 1024;
pub const CSV_COLUMNS: [&str; 10] = [
    "format",
    "external_id",
    "date",
    "kind",
    "account",
    "destination",
    "amount",
    "currency",
    "category",
    "note",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFileKind {
    Csv,
    Backup,
}
impl ImportFileKind {
    pub const fn max_bytes(self) -> usize {
        match self {
            Self::Csv => MAX_CSV_BYTES,
            Self::Backup => MAX_BACKUP_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentExport {
    pub filename: String,
    pub mime: &'static str,
    pub bytes: Arc<[u8]>,
}
impl From<CsvExport> for DocumentExport {
    fn from(csv: CsvExport) -> Self {
        Self {
            filename: csv.filename,
            mime: "text/csv",
            bytes: csv.contents.into_bytes().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupSummary {
    pub accounts: usize,
    pub transactions: usize,
    pub plans: usize,
    pub receivables: usize,
    pub currency: Currency,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupReview {
    pub bytes: Arc<[u8]>,
    pub summary: BackupSummary,
}

pub fn csv_template(today: EntryDate, currency: Currency) -> CsvExport {
    // Authored examples only: never place user-controlled spreadsheet formulas in a template.
    CsvExport {
        filename: "LedgersBro-import-template.csv".into(),
        contents: format!(
            "\u{feff}{}\r\nledgers-bro-v1,coffee-001,{today},expense,เงินสด,,80.00,{},food,กาแฟ\r\nledgers-bro-v1,salary-001,{today},income,ธนาคาร,,30000.00,{},salary,เงินเดือน\r\nledgers-bro-v1,transfer-001,{today},transfer,ธนาคาร,เงินสด,1000.00,{},,ถอนเงินสด\r\n",
            CSV_COLUMNS.join(","),
            currency.code(),
            currency.code(),
            currency.code()
        ),
    }
}

/// CSV is an explicit command format, not an AI inference or a bank statement guess.
/// Stable external IDs give identical retries the repository's exactly-once semantics.
pub fn prepare_csv_import(
    bytes: &[u8],
    state: &LedgerState,
    today: EntryDate,
) -> Result<Vec<PreparedEntry>, AppError> {
    if bytes.is_empty() || bytes.len() > MAX_CSV_BYTES {
        return Err(AppError::Input("ไฟล์ CSV ต้องไม่เกิน 4 MB".into()));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| AppError::Input("CSV ต้องเป็น UTF-8 ตามไฟล์ตัวอย่าง".into()))?
        .trim_start_matches('\u{feff}');
    validate_csv_quotes(text)?;
    let mut reader = csv::ReaderBuilder::new()
        .flexible(false)
        .from_reader(text.as_bytes());
    let invalid = || AppError::Input("รูปแบบ CSV ไม่ตรงแม่แบบ กรุณาใช้ไฟล์ตัวอย่างของ Ledgers Bro".into());
    if reader
        .headers()
        .map_err(|_| invalid())?
        .iter()
        .ne(CSV_COLUMNS)
    {
        return Err(invalid());
    }
    let mut result = Vec::new();
    let mut ids = BTreeSet::new();
    let mut staged = state.clone();
    for (index, row) in reader.records().enumerate() {
        if index >= MAX_IMPORT_ROWS {
            return Err(AppError::Input("นำเข้า CSV ได้ครั้งละไม่เกิน 1000 รายการ".into()));
        }
        let row = row.map_err(|_| {
            AppError::Input(format!(
                "CSV record {}: invalid columns or quoting",
                index + 2
            ))
        })?;
        let parse = || -> Result<PreparedEntry, AppError> {
            let cell = |n: usize| row.get(n).ok_or_else(invalid);
            if cell(0)? != "ledgers-bro-v1" || cell(7)? != state.currency.code() {
                return Err(invalid());
            }
            let external = cell(1)?.trim();
            if external.is_empty() || external.len() > 100 || external.chars().any(char::is_control)
            {
                return Err(AppError::Input(
                    "external_id must be a unique identifier of 1-100 bytes".into(),
                ));
            }
            let uuid = uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_URL,
                format!("ledgers-bro/csv/v1/{external}").as_bytes(),
            );
            let id = EntryId::from_uuid(uuid)?;
            let submission = SubmissionId::from_uuid(uuid)?;
            let account = |text: &str| -> Result<AccountId, AppError> {
                let name = AccountName::new(text)?;
                state
                    .accounts
                    .iter()
                    .find(|a| a.name().key() == name.key() && a.accepts_cash_entries())
                    .map(Account::id)
                    .ok_or_else(|| {
                        AppError::Input(format!("Account not found or unavailable: {text}"))
                    })
            };
            let source = account(cell(4)?)?;
            let amount = PositiveMoney::new(cell(6)?.parse()?)?;
            let date: EntryDate = cell(2)?.parse()?;
            if date > today {
                return Err(AppError::Input(
                    "Transaction date must not be after today".into(),
                ));
            }
            let kind = match cell(3)? {
                "expense" | "income" => {
                    if !cell(5)?.is_empty() {
                        return Err(invalid());
                    }
                    let category = Category::from_code(cell(8)?)?;
                    if cell(3)? == "expense" {
                        EntryKind::Expense {
                            account: source,
                            amount,
                            category,
                        }
                    } else {
                        EntryKind::Income {
                            account: source,
                            amount,
                            category,
                        }
                    }
                }
                "transfer" => {
                    if !cell(8)?.is_empty() {
                        return Err(invalid());
                    }
                    EntryKind::Transfer {
                        from: source,
                        to: account(cell(5)?)?,
                        amount,
                    }
                }
                _ => return Err(invalid()),
            };
            Ok(PreparedEntry {
                entry: JournalEntry::record(id, date, Note::new(cell(9)?)?, kind)?,
                submission,
                recurring: None,
            })
        };
        let prepared = parse()
            .map_err(|error| AppError::Input(format!("CSV record {}: {error}", index + 2)))?;
        if !ids.insert(prepared.entry.id()) {
            return Err(AppError::Input(format!(
                "CSV record {}: duplicate external_id",
                index + 2
            )));
        }
        if let Some(existing) = staged
            .entries
            .iter()
            .find(|e| e.id() == prepared.entry.id())
        {
            if existing != &prepared.entry {
                return Err(StorageError::SubmissionConflict.into());
            }
        } else {
            validate_append(&staged, &prepared.entry)
                .map_err(|error| AppError::Input(format!("CSV record {}: {error}", index + 2)))?;
            staged.entries.push(prepared.entry.clone());
        }
        result.push(prepared);
    }
    if result.is_empty() {
        return Err(AppError::Input("CSV ไม่มีรายการให้นำเข้า".into()));
    }
    Ok(result)
}

fn validate_csv_quotes(text: &str) -> Result<(), AppError> {
    // The csv crate intentionally accepts loose quoting. Our authored import
    // format does not: reject truncated quotes and text after a closing quote.
    let mut state = 0u8; // field start, bare field, quoted field, after quote
    for byte in text.bytes() {
        state = match (state, byte) {
            (0, b'"') => 2,
            (0 | 1, b',' | b'\r' | b'\n') => 0,
            (1, b'"') => return Err(AppError::Input("CSV มีเครื่องหมายคำพูดไม่ถูกต้อง".into())),
            (0 | 1, _) => 1,
            (2, b'"') => 3,
            (2, _) => 2,
            (3, b'"') => 2,
            (3, b',' | b'\r' | b'\n') => 0,
            _ => return Err(AppError::Input("CSV มีเครื่องหมายคำพูดไม่ถูกต้อง".into())),
        };
    }
    if state == 2 {
        return Err(AppError::Input("CSV มีเครื่องหมายคำพูดไม่ครบ".into()));
    }
    Ok(())
}
