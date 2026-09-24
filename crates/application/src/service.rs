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
    pub recurring: Option<RecurringSelection>,
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
            recurring: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEntry {
    pub submission: SubmissionId,
    pub entry: JournalEntry,
    pub recurring: Option<PreparedRecurringLink>,
}

#[derive(Debug, Clone)]
pub enum Command {
    LoadPreferences,
    SetPreferences(UserPreferences),
    SetCurrency(Currency),
    SetThaiTaxEnabled(bool),
    PreviewPrompt(Vec<PromptDraft>),
    CommitPrompt(PromptPlan),
    PreviewReceivable(ReceivableInput),
    CreateReceivable(PreparedReceivable),
    PreviewRepayment(RepaymentInput),
    ReceiveRepayment(Vec<PreparedEntry>),
    Load,
    AddRecurring(RecurringInput),
    SetRecurringInstallments {
        expected: RecurringExpense,
        installments: Option<String>,
    },
    StopRecurring {
        expected: RecurringExpense,
        month: Month,
    },
    PreviewRecurringPayment {
        expected: RecurringExpense,
        month: Month,
        input: EntryInput,
    },
    PayRecurring(PreparedRecurringPayment),
    LinkRecurring {
        expected: RecurringExpense,
        month: Month,
        entry: EntryId,
    },
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
    PreviewBatch(Vec<EntryInput>),
    CommitBatch(Vec<PreparedEntry>),
    Reverse(EntryId),
    ExportCsv(ExportOptions),
}

#[derive(Debug, Clone)]
pub enum Response {
    Preferences(UserPreferences),
    PreparedPrompt(PromptPlan),
    PromptCommitted,
    ReceivableReview(ReceivableReview),
    Dashboard(Dashboard),
    RecurringChanged,
    PreparedRecurringPayment(PreparedRecurringPayment),
    AccountDeletion(AccountDeletion),
    AccountDeleted(AccountId),
    Resolved(QuickResolution),
    Prepared(PreparedEntry),
    Committed(CommitOutcome),
    PreparedBatch(Vec<PreparedEntry>),
    CommittedBatch(Vec<CommitOutcome>),
    Csv(CsvExport),
}

pub struct LedgerApplication<R, C, I> {
    repository: R,
    clock: C,
    ids: I,
}
impl<R: LedgerRepository, C: Clock, I: IdSource> LedgerApplication<R, C, I> {
    pub(crate) fn into_repository(self) -> R {
        self.repository
    }
    pub fn new(repository: R, clock: C, ids: I) -> Self {
        Self {
            repository,
            clock,
            ids,
        }
    }
    pub fn execute(&mut self, command: Command) -> Result<Response, AppError> {
        match command {
            Command::LoadPreferences => Ok(Response::Preferences(self.repository.preferences()?)),
            Command::SetPreferences(preferences) => {
                if preferences.primary_color > 0xffffff {
                    return Err(AppError::Input("Invalid theme color".into()));
                }
                self.repository.set_preferences(preferences)?;
                Ok(Response::Preferences(self.repository.preferences()?))
            }
            Command::SetThaiTaxEnabled(enabled) => {
                self.repository.set_thai_tax_enabled(enabled)?;
                self.execute(Command::Load)
            }
            Command::SetCurrency(currency) => {
                self.repository.set_currency(currency)?;
                self.execute(Command::Load)
            }
            Command::PreviewPrompt(drafts) => Ok(Response::PreparedPrompt(prepare_prompt(
                drafts,
                self.repository.snapshot()?,
                self.clock.today()?,
                &self.ids,
            )?)),
            Command::CommitPrompt(plan) => {
                self.repository.commit_prompt(&plan)?;
                Ok(Response::PromptCommitted)
            }
            Command::PreviewReceivable(input) => {
                let date: EntryDate = input.opened.parse()?;
                if date > self.clock.today()? {
                    return Err(AppError::Input("วันที่ตั้งหนี้ต้องไม่เกินวันนี้".into()));
                }
                let loan = Receivable::new(
                    self.ids.receivable_id()?,
                    AccountName::new(&input.debtor)?,
                    Note::new(&input.description)?,
                    PositiveMoney::new(input.total.parse()?)?,
                    date,
                    CollectionTerms::new(
                        input.start.parse()?,
                        input
                            .day
                            .as_deref()
                            .map(|v| v.parse().map_err(|_| DomainError::InvalidRecurring))
                            .transpose()?,
                        parse_installments(input.installments.as_deref())?,
                    )?,
                )?;
                let kind = match input.source {
                    Some(account) => EntryKind::Lending {
                        receivable: loan.id(),
                        account,
                        amount: loan.total(),
                    },
                    None => EntryKind::ReceivableOpening {
                        receivable: loan.id(),
                        amount: loan.total(),
                    },
                };
                let entry = JournalEntry::record(
                    self.ids.entry_id()?,
                    date,
                    Note::new(&format!(
                        "ลูกหนี้ {} · {}",
                        loan.debtor().as_str(),
                        loan.description().as_str()
                    ))?,
                    kind,
                )?;
                let mut state = self.repository.snapshot()?;
                state.receivables.push(loan.clone());
                validate_append(&state, &entry)?;
                Ok(Response::ReceivableReview(ReceivableReview::New(Box::new(
                    PreparedReceivable {
                        loan,
                        opening: PreparedEntry {
                            submission: self.ids.submission_id()?,
                            entry,
                            recurring: None,
                        },
                    },
                ))))
            }
            Command::CreateReceivable(prepared) => {
                if prepared.loan.opened() > self.clock.today()? {
                    return Err(AppError::Input("วันที่ตั้งหนี้ต้องไม่เกินวันนี้".into()));
                }
                Ok(Response::Committed(
                    self.repository.create_receivable(&prepared)?,
                ))
            }
            Command::PreviewRepayment(input) => {
                let mut state = self.repository.snapshot()?;
                let loan = state
                    .receivables
                    .iter()
                    .find(|r| r.id() == input.receivable)
                    .ok_or(StorageError::ReceivableChanged)?
                    .clone();
                let date: EntryDate = input.date.parse()?;
                if date > self.clock.today()? {
                    return Err(AppError::Input("วันที่รับชำระต้องไม่เกินวันนี้".into()));
                }
                let account = input
                    .account
                    .ok_or_else(|| AppError::Input("กรุณาเลือกบัญชีรับเงิน".into()))?;
                let principal = PositiveMoney::new(input.principal.parse()?)?;
                let interest: Money = if input.interest.trim().is_empty() {
                    Money::ZERO
                } else {
                    input.interest.parse()?
                };
                if interest < Money::ZERO {
                    return Err(DomainError::NonPositiveAmount.into());
                }
                let mut kinds = vec![EntryKind::Repayment {
                    receivable: loan.id(),
                    account,
                    amount: principal,
                }];
                if interest > Money::ZERO {
                    kinds.push(EntryKind::Income {
                        account,
                        amount: PositiveMoney::new(interest)?,
                        category: Category::Interest,
                    });
                }
                let mut prepared = Vec::new();
                for kind in kinds {
                    let label = if matches!(kind, EntryKind::Repayment { .. }) {
                        "รับคืนเงินต้น"
                    } else {
                        "รับดอกเบี้ย"
                    };
                    let entry = JournalEntry::record(
                        self.ids.entry_id()?,
                        date,
                        Note::new(&format!(
                            "{label}: {} · {}",
                            loan.debtor().as_str(),
                            loan.description().as_str()
                        ))?,
                        kind,
                    )?;
                    validate_append(&state, &entry)?;
                    state.entries.push(entry.clone());
                    prepared.push(PreparedEntry {
                        entry,
                        submission: self.ids.submission_id()?,
                        recurring: None,
                    });
                }
                Ok(Response::ReceivableReview(ReceivableReview::Payment(
                    prepared,
                )))
            }
            Command::ReceiveRepayment(prepared) => {
                let valid = match prepared.as_slice() {
                    [principal] => matches!(principal.entry.kind(), EntryKind::Repayment { .. }),
                    [principal, interest] => {
                        matches!((principal.entry.kind(), interest.entry.kind()),
                        (EntryKind::Repayment { account: a, .. }, EntryKind::Income { account: b, category: Category::Interest, .. }) if a == b && principal.entry.date() == interest.entry.date())
                    }
                    _ => false,
                };
                if !valid {
                    return Err(AppError::Input("กรุณาตรวจรายการรับชำระใหม่".into()));
                }
                self.execute(Command::CommitBatch(prepared))
            }
            Command::SetRecurringInstallments {
                expected,
                installments,
            } => {
                self.repository.set_recurring_installments(
                    &expected,
                    parse_installments(installments.as_deref())?,
                )?;
                Ok(Response::RecurringChanged)
            }
            Command::AddRecurring(input) => {
                let schedule = input.validate(self.ids.recurring_id()?)?;
                if schedule.account().is_none() {
                    return Err(AppError::Input("กรุณาเลือกบัญชีสำหรับรายจ่ายประจำ".into()));
                }
                self.repository.add_recurring(&schedule)?;
                Ok(Response::RecurringChanged)
            }
            Command::StopRecurring { expected, month } => {
                self.repository.stop_recurring(&expected, month)?;
                Ok(Response::RecurringChanged)
            }
            Command::PreviewRecurringPayment {
                expected,
                month,
                input,
            } => {
                let Response::Prepared(payment) = self.execute(Command::Preview(input))? else {
                    return Err(AppError::Input("ตรวจรายการไม่สำเร็จ".into()));
                };
                validate_recurring_settlement(
                    &self.repository.snapshot()?,
                    &expected,
                    month,
                    &payment.entry,
                )?;
                Ok(Response::PreparedRecurringPayment(
                    PreparedRecurringPayment {
                        schedule: expected,
                        month,
                        payment,
                    },
                ))
            }
            Command::PayRecurring(prepared) => {
                if prepared.payment.entry.date() > self.clock.today()? {
                    return Err(AppError::Input("บันทึกจ่ายได้ถึงวันนี้".into()));
                }
                Ok(Response::Committed(
                    self.repository.pay_recurring(&prepared)?,
                ))
            }
            Command::LinkRecurring {
                expected,
                month,
                entry,
            } => {
                self.repository.link_recurring(&expected, month, entry)?;
                Ok(Response::RecurringChanged)
            }
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
            Command::Resolve(text) => Ok(Response::Resolved(resolve_prompt_text(
                &text,
                self.clock.today()?,
                &self.repository.snapshot()?,
            )?)),
            Command::ResolveModel { source, output } => {
                let state = self.repository.snapshot()?;
                let source = normalize_prompt_currency(&source, state.currency)?;
                Ok(Response::Resolved(resolve_model_proposal(
                    &source,
                    &output,
                    self.clock.today()?,
                    &state.accounts,
                )?))
            }
            Command::Preview(input) => {
                let note = if let Some(receipt) = &input.receipt {
                    if self.repository.snapshot()?.currency != Currency::Thb {
                        return Err(AppError::Input("Receipt OCR currently supports THB ledgers only. Use manual entry in your ledger currency.".into()));
                    }
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
                let state = self.repository.snapshot()?;
                entry.validate_accounts(&state.accounts)?;
                let recurring = if let Some(selection) = input.recurring {
                    let schedule = state
                        .recurring
                        .iter()
                        .find(|s| s.id() == selection.recurring)
                        .ok_or(StorageError::RecurringChanged)?;
                    let month: Month = selection
                        .month
                        .parse()
                        .map_err(|_| AppError::Input("งวดเดือนต้องเป็น YYYY-MM (ค.ศ.)".into()))?;
                    validate_recurring_settlement(&state, schedule, month, &entry)?;
                    Some(PreparedRecurringLink {
                        schedule: schedule.clone(),
                        month,
                    })
                } else {
                    None
                };
                Ok(Response::Prepared(PreparedEntry {
                    submission: self.ids.submission_id()?,
                    entry,
                    recurring,
                }))
            }
            Command::Commit(prepared) => {
                if matches!(
                    prepared.entry.kind(),
                    EntryKind::Opening { .. }
                        | EntryKind::ReceivableOpening { .. }
                        | EntryKind::Lending { .. }
                        | EntryKind::Reversal { .. }
                ) {
                    return Err(AppError::Input("รายการนี้ต้องใช้ขั้นตอนเฉพาะ".into()));
                }
                // Journal + selected installment are saved by one atomic batch.
                if prepared.recurring.is_some() {
                    return Ok(Response::Committed(
                        self.repository
                            .commit_batch(&[prepared])?
                            .into_iter()
                            .next()
                            .ok_or(StorageError::Corrupt)?,
                    ));
                }
                // Repository repeats account checks inside the write transaction.
                Ok(Response::Committed(
                    self.repository
                        .commit(&prepared.entry, prepared.submission)?,
                ))
            }
            Command::PreviewBatch(inputs) => {
                if inputs.is_empty() || inputs.len() > MAX_BATCH_ENTRIES {
                    return Err(AppError::Input("ตรวจได้ครั้งละ 1–8 รายการ".into()));
                }
                let mut prepared = Vec::new();
                let mut selected = std::collections::BTreeSet::new();
                for input in inputs {
                    if let Some(s) = &input.recurring
                        && !selected.insert((s.recurring, s.month.clone()))
                    {
                        return Err(AppError::Input(
                            "เลือกงวดเดียวกันซ้ำในชุดนี้ กรุณาเลือกหนึ่งรายการต่อหนึ่งงวด".into(),
                        ));
                    }
                    let Response::Prepared(entry) = self.execute(Command::Preview(input))? else {
                        return Err(AppError::Input("ตรวจรายการไม่สำเร็จ".into()));
                    };
                    prepared.push(entry);
                }
                Ok(Response::PreparedBatch(prepared))
            }
            Command::CommitBatch(prepared) => {
                if prepared.is_empty()
                    || prepared.len() > MAX_BATCH_ENTRIES
                    || prepared.iter().any(|p| {
                        matches!(
                            p.entry.kind(),
                            EntryKind::Opening { .. }
                                | EntryKind::ReceivableOpening { .. }
                                | EntryKind::Lending { .. }
                                | EntryKind::Reversal { .. }
                        )
                    })
                {
                    return Err(AppError::Input("ชุดรายการไม่ถูกต้อง กรุณาตรวจรายการใหม่".into()));
                }
                Ok(Response::CommittedBatch(
                    self.repository.commit_batch(&prepared)?,
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
