use crate::*;
use ledger_domain::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionKind {
    Expense,
    Income,
    Transfer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryInput {
    pub kind: TransactionKind,
    pub amount: String,
    pub account: Option<AccountId>,
    pub destination: Option<AccountId>,
    pub category: Option<Category>,
    pub date: String,
    pub note: String,
    pub receipt: Option<ReceiptInput>,
}

impl EntryInput {
    /// Presentation completeness only. Preview/commit still validate every value.
    pub fn is_complete(&self) -> bool {
        !self.amount.trim().is_empty()
            && !self.date.trim().is_empty()
            && self.account.is_some()
            && self
                .receipt
                .as_ref()
                .is_none_or(|receipt| receipt.verified(&self.amount).is_ok())
            && match self.kind {
                TransactionKind::Transfer => self.destination.is_some(),
                TransactionKind::Expense | TransactionKind::Income => self.category.is_some(),
            }
    }

    pub fn empty(today: EntryDate) -> Self {
        Self {
            kind: TransactionKind::Expense,
            amount: String::new(),
            account: None,
            destination: None,
            category: None,
            date: today.to_string(),
            note: String::new(),
            receipt: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEntry {
    pub submission: SubmissionId,
    pub entry: JournalEntry,
}

#[derive(Debug, Clone)]
pub enum Command {
    Load,
    PreviewDeleteAccount(AccountId),
    DeleteAccount(AccountDeletion),
    CreateAccount {
        name: String,
        kind: AccountKind,
        opening: String,
    },
    Resolve(String),
    ResolveModel {
        source: String,
        output: String,
    },
    Preview(EntryInput),
    Commit(PreparedEntry),
    Reverse(EntryId),
    ExportCsv(ExportOptions),
}

#[derive(Debug, Clone)]
pub enum Response {
    Dashboard(Dashboard),
    AccountDeletion(AccountDeletion),
    AccountDeleted(AccountId),
    Resolved(QuickResolution),
    Prepared(PreparedEntry),
    Committed(CommitOutcome),
    Csv(CsvExport),
}

pub struct LedgerApplication<R, C, I> {
    repository: R,
    clock: C,
    ids: I,
}
impl<R: LedgerRepository, C: Clock, I: IdSource> LedgerApplication<R, C, I> {
    pub fn new(repository: R, clock: C, ids: I) -> Self {
        Self {
            repository,
            clock,
            ids,
        }
    }
    pub fn execute(&mut self, command: Command) -> Result<Response, AppError> {
        match command {
            Command::PreviewDeleteAccount(id) => Ok(Response::AccountDeletion(
                AccountDeletion::prepare(self.repository.snapshot()?, id, self.clock.today()?)?,
            )),
            Command::DeleteAccount(deletion) => {
                self.repository.delete_account(&deletion)?;
                Ok(Response::AccountDeleted(deletion.account().id()))
            }
            Command::Load => Ok(Response::Dashboard(dashboard(
                self.repository.snapshot()?,
                self.clock.today()?,
            )?)),
            Command::CreateAccount {
                name,
                kind,
                opening,
            } => {
                let account = Account::new(self.ids.account_id()?, AccountName::new(&name)?, kind);
                let state = self.repository.snapshot()?;
                account.ensure_can_add(&state.accounts)?;
                let entered: Money = opening.parse()?;
                // UI asks for amount owed on credit cards, not a credit limit.
                let balance = if kind == AccountKind::CreditCard {
                    entered.negated()
                } else {
                    entered
                };
                let entry = JournalEntry::record(
                    self.ids.entry_id()?,
                    self.clock.today()?,
                    Note::new("ยอดเริ่มต้น")?,
                    EntryKind::Opening {
                        account: account.id(),
                        balance,
                    },
                )?;
                Ok(Response::Committed(self.repository.create_account(
                    &account,
                    &entry,
                    self.ids.submission_id()?,
                )?))
            }
            Command::Resolve(text) => Ok(Response::Resolved(resolve_quick_entry(
                &text,
                self.clock.today()?,
                &self.repository.snapshot()?.accounts,
            )?)),
            Command::ResolveModel { source, output } => {
                Ok(Response::Resolved(resolve_model_proposal(
                    &source,
                    &output,
                    self.clock.today()?,
                    &self.repository.snapshot()?.accounts,
                )?))
            }
            Command::Preview(input) => {
                let note = if let Some(receipt) = &input.receipt {
                    if input.kind == TransactionKind::Transfer {
                        return Err(AppError::Input(
                            "รายการจากใบเสร็จใช้รายรับหรือรายจ่าย ไม่ใช้การโอน".into(),
                        ));
                    }
                    receipt.verified(&input.amount)?.note(&input.note)?
                } else {
                    Note::new(&input.note)?
                };
                let date: EntryDate = input.date.parse()?;
                if date > self.clock.today()? {
                    return Err(AppError::Input(
                        "รายการล่วงหน้าให้ใช้ระบบกำหนดชำระ รุ่นนี้บันทึกได้ถึงวันนี้".into(),
                    ));
                }
                let amount = PositiveMoney::new(input.amount.parse()?)?;
                let account = input
                    .account
                    .ok_or_else(|| AppError::Input("กรุณาเลือกบัญชี".into()))?;
                let kind = match input.kind {
                    TransactionKind::Expense => EntryKind::Expense {
                        account,
                        amount,
                        category: input.category.ok_or(DomainError::InvalidCategory)?,
                    },
                    TransactionKind::Income => EntryKind::Income {
                        account,
                        amount,
                        category: input.category.ok_or(DomainError::InvalidCategory)?,
                    },
                    TransactionKind::Transfer => EntryKind::Transfer {
                        from: account,
                        to: input
                            .destination
                            .ok_or_else(|| AppError::Input("กรุณาเลือกบัญชีปลายทาง".into()))?,
                        amount,
                    },
                };
                if input.kind != TransactionKind::Transfer && input.destination.is_some() {
                    return Err(AppError::Input("รายการนี้ไม่ใช้บัญชีปลายทาง".into()));
                }
                if input.kind == TransactionKind::Transfer && input.category.is_some() {
                    return Err(AppError::Input("การโอนไม่ใช้หมวดรายรับรายจ่าย".into()));
                }
                let entry = JournalEntry::record(self.ids.entry_id()?, date, note, kind)?;
                entry.validate_accounts(&self.repository.snapshot()?.accounts)?;
                Ok(Response::Prepared(PreparedEntry {
                    submission: self.ids.submission_id()?,
                    entry,
                }))
            }
            Command::Commit(prepared) => {
                if matches!(
                    prepared.entry.kind(),
                    EntryKind::Opening { .. } | EntryKind::Reversal { .. }
                ) {
                    return Err(AppError::Input("รายการนี้ต้องใช้ขั้นตอนเฉพาะ".into()));
                }
                // Repository repeats account checks inside the write transaction.
                Ok(Response::Committed(
                    self.repository
                        .commit(&prepared.entry, prepared.submission)?,
                ))
            }
            Command::Reverse(id) => {
                let state = self.repository.snapshot()?;
                let original = state
                    .entries
                    .iter()
                    .find(|entry| entry.id() == id)
                    .ok_or(DomainError::InvalidReversal)?;
                let entry = JournalEntry::reverse(
                    self.ids.entry_id()?,
                    original,
                    Note::new("ยกเลิกรายการโดยผู้ใช้")?,
                )?;
                Ok(Response::Committed(
                    self.repository.commit(&entry, self.ids.submission_id()?)?,
                ))
            }
            Command::ExportCsv(options) => Ok(Response::Csv(export_csv(
                &dashboard(self.repository.snapshot()?, self.clock.today()?)?,
                options,
            )?)),
        }
    }
}
