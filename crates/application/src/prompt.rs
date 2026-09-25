//! One editable action vocabulary for everything the ledger can persist.
//! Interpretation produces drafts; preview runs the normal use cases in memory.
use crate::*;
use ledger_domain::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptKind {
    Expense,
    Income,
    Transfer,
    Account,
    Recurring,
    PayRecurring,
    LinkRecurring,
    CountRecurring,
    StopRecurring,
    Receivable,
    Lending,
    Repayment,
    Reverse,
    DeleteAccount,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptFieldType {
    Text,
    Money,
    Date,
    Month,
    Account,
    AccountKind,
    ExpenseCategory,
    IncomeCategory,
    Recurring,
    Receivable,
    Entry,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptField {
    pub key: &'static str,
    pub label: &'static str,
    pub required: bool,
    pub kind: PromptFieldType,
}
impl PromptKind {
    pub const ALL: [Self; 14] = [
        Self::Expense,
        Self::Income,
        Self::Transfer,
        Self::Account,
        Self::Recurring,
        Self::PayRecurring,
        Self::LinkRecurring,
        Self::CountRecurring,
        Self::StopRecurring,
        Self::Receivable,
        Self::Lending,
        Self::Repayment,
        Self::Reverse,
        Self::DeleteAccount,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Expense => "รายจ่าย",
            Self::Income => "รายรับ",
            Self::Transfer => "โอนเงิน",
            Self::Account => "เพิ่มบัญชี",
            Self::Recurring => "เพิ่มรายจ่ายประจำ",
            Self::PayRecurring => "จ่ายงวดรายจ่ายประจำ",
            Self::LinkRecurring => "ผูกรายจ่ายเดิมกับงวด",
            Self::CountRecurring => "ตั้งจำนวนงวด",
            Self::StopRecurring => "หยุดแผนรายจ่าย",
            Self::Receivable => "เพิ่มลูกหนี้ที่ค้างอยู่",
            Self::Lending => "ให้ยืมเงินใหม่",
            Self::Repayment => "ลูกหนี้ชำระหนี้",
            Self::Reverse => "ยกเลิกรายการ",
            Self::DeleteAccount => "ลบบัญชีและรายการที่เกี่ยวข้อง",
        }
    }
    pub fn example(self) -> &'static str {
        match self {
            Self::Expense => "จ่าย 80 จาก เงินสด หมวด อาหาร",
            Self::Income => "รับ 400 เข้า กรุงไทย หมวด ฟรีแลนซ์",
            Self::Transfer => "โอน 1000 จาก กรุงไทย ไป เงินสด",
            Self::Account => "เพิ่มบัญชี ชื่อ ออม ประเภท ธนาคาร ยอดเริ่มต้น 5000",
            Self::Recurring => {
                "เพิ่มรายจ่ายประจำ ชื่อ ค่ารถ ยอด 4500 บาท ทุกวันที่ 5 จำนวนงวด 12 เริ่ม 2026-10 จาก เงินสด หมวด อื่นๆ"
            }
            Self::PayRecurring => "จ่ายงวด ชื่อ ค่ารถ ยอด 4500 บาท จาก เงินสด งวด 2026-10",
            Self::LinkRecurring => "ผูกรายจ่ายเดิม ชื่อ ค่ารถ งวด 2026-10 รายการ ค่ารถ",
            Self::CountRecurring => "ตั้งจำนวนงวด ชื่อ ค่ารถ จำนวนงวด 12",
            Self::StopRecurring => "หยุดแผน ชื่อ ค่ารถ ตั้งแต่ 2026-10",
            Self::Receivable => {
                "เพิ่มลูกหนี้ ชื่อ สมชาย เรื่อง ยืมซื้อคอม ยอด 9000 บาท จำนวนงวด 9 เก็บทุกวันที่ 5 เริ่ม 2026-10"
            }
            Self::Lending => {
                "ให้สมชายยืม 9000 บาท จาก เงินสด เรื่อง ยืมซื้อคอม จำนวนงวด 9 เก็บทุกวันที่ 5 เริ่ม 2026-10"
            }
            Self::Repayment => "สมชายคืนหนี้ 1000 บาท เข้า กรุงไทย ดอกเบี้ย 50 บาท",
            Self::Reverse => "ยกเลิกรายการ รายการ กาแฟ วันที่ 2026-09-24 ยอด 80 บาท",
            Self::DeleteAccount => "ลบบัญชี ชื่อ ออม",
        }
    }
    pub fn fields(self) -> Vec<PromptField> {
        use PromptFieldType as T;
        let f = |key, label, required, kind| PromptField {
            key,
            label,
            required,
            kind,
        };
        let amount = f("amount", "ยอดเงิน (บาท)", true, T::Money);
        let account = f("account", "บัญชี", true, T::Account);
        let date = f("date", "วันที่รายการ", true, T::Date);
        let note = f("note", "รายละเอียด", false, T::Text);
        let plan = f("plan", "แผนรายจ่ายประจำ", true, T::Recurring);
        let month = f("month", "งวดเดือน (ค.ศ.)", true, T::Month);
        let count = f("count", "จำนวนงวด หรือ ไม่กำหนด", false, T::Text);
        match self {
            Self::Expense | Self::Income => vec![
                amount,
                account,
                f(
                    "category",
                    "หมวดหมู่",
                    true,
                    if self == Self::Expense {
                        T::ExpenseCategory
                    } else {
                        T::IncomeCategory
                    },
                ),
                date,
                note,
            ],
            Self::Transfer => vec![
                amount,
                account,
                f("destination", "บัญชีปลายทาง", true, T::Account),
                date,
                note,
            ],
            Self::Account => vec![
                f("name", "ชื่อบัญชี", true, T::Text),
                f("kind", "ประเภทบัญชี", true, T::AccountKind),
                f("opening", "ยอดเริ่มต้น / ยอดหนี้บัตร (บาท)", true, T::Money),
                f("closing_day", "วันตัดรอบบัตรเครดิต (1–31)", false, T::Text),
                f("payment_day", "วันชำระบัตรเครดิต (1–31)", false, T::Text),
            ],
            Self::Recurring => vec![
                f("name", "ชื่อแผน", true, T::Text),
                amount,
                f("day", "ทุกวันที่ (1–31)", true, T::Text),
                f("start", "เดือนเริ่ม (ค.ศ.)", true, T::Month),
                count,
                account,
                f("category", "หมวดรายจ่าย", true, T::ExpenseCategory),
            ],
            Self::PayRecurring => vec![plan, month, amount, account, date],
            Self::LinkRecurring => vec![
                plan,
                month,
                f("entry", "รายจ่ายเดิมที่ต้องการผูก", true, T::Entry),
            ],
            Self::CountRecurring => vec![plan, f("count", "จำนวนงวด หรือ ไม่กำหนด", true, T::Text)],
            Self::StopRecurring => vec![plan, month],
            Self::Receivable | Self::Lending => {
                let mut fields = vec![
                    f("debtor", "ชื่อลูกหนี้", true, T::Text),
                    f("description", "หนี้อะไร", true, T::Text),
                    amount,
                    date,
                    f("start", "เดือนเริ่มเก็บ (ค.ศ.)", true, T::Month),
                    count,
                    f("day", "เก็บทุกวันที่ หรือ ไม่กำหนด", false, T::Text),
                ];
                if self == Self::Lending {
                    fields.push(account);
                }
                fields
            }
            Self::Repayment => vec![
                f("loan", "ลูกหนี้ / เรื่องหนี้", true, T::Receivable),
                amount,
                f("interest", "ดอกเบี้ยรับ (ไม่มีใส่ 0)", false, T::Money),
                account,
                date,
            ],
            Self::Reverse => vec![
                f("entry", "รายการที่จะยกเลิก", true, T::Entry),
                f("date", "วันที่ใช้ค้นหารายการ (เว้นว่างได้)", false, T::Date),
                f("amount", "ยอดใช้ค้นหารายการ (เว้นว่างได้)", false, T::Money),
            ],
            Self::DeleteAccount => vec![account],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptDraft {
    pub kind: PromptKind,
    pub fields: BTreeMap<String, String>,
    pub source: String,
    pub issues: BTreeMap<String, String>,
}
impl PromptDraft {
    pub fn empty(kind: PromptKind, today: EntryDate) -> Self {
        let mut fields = BTreeMap::new();
        if kind.fields().iter().any(|f| f.key == "date" && f.required) {
            fields.insert("date".into(), today.to_string());
        }
        if kind.fields().iter().any(|f| f.key == "start") {
            fields.insert(
                "start".into(),
                format!("{:04}-{:02}", today.month_key().0, today.month_key().1),
            );
        }
        Self {
            kind,
            fields,
            source: String::new(),
            issues: BTreeMap::new(),
        }
    }
    pub fn get(&self, key: &str) -> &str {
        self.fields.get(key).map_or("", String::as_str)
    }
    pub fn set(&mut self, key: &str, value: String) {
        self.fields.insert(key.into(), value);
        self.issues.remove(key);
    }
    pub fn questions(&self) -> Vec<String> {
        let mut questions: Vec<_> = self
            .kind
            .fields()
            .into_iter()
            .filter(|f| f.required && self.get(f.key).trim().is_empty())
            .map(|f| format!("กรุณาระบุ{}", f.label))
            .collect();
        questions.extend(self.issues.values().cloned());
        questions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptPlan {
    id: SubmissionId,
    before: LedgerState,
    after: LedgerState,
    entries: Vec<PreparedEntry>,
    pub descriptions: Vec<String>,
}
impl PromptPlan {
    pub fn id(&self) -> SubmissionId {
        self.id
    }
    pub fn before(&self) -> &LedgerState {
        &self.before
    }
    pub fn after(&self) -> &LedgerState {
        &self.after
    }
    pub fn entries(&self) -> &[PreparedEntry] {
        &self.entries
    }
    pub fn validate_current(&self, state: &LedgerState) -> Result<(), StorageError> {
        if state != &self.before {
            Err(StorageError::PromptChanged)
        } else {
            Ok(())
        }
    }
}

fn input_error(text: impl Into<String>) -> AppError {
    AppError::Input(text.into())
}
fn optional_count(draft: &PromptDraft) -> Result<Option<String>, AppError> {
    let value = draft.get("count").trim();
    if value.is_empty() || value == "ไม่กำหนด" || value == "ไม่จำกัด"
    {
        Ok(None)
    } else {
        parse_installments(Some(value))?;
        Ok(Some(value.into()))
    }
}
fn account_id(state: &LedgerState, value: &str) -> Result<AccountId, AppError> {
    state
        .accounts
        .iter()
        .find(|a| {
            !a.is_archived()
                && (a.id().to_string() == value || a.name().key() == value.trim().to_lowercase())
        })
        .map(Account::id)
        .ok_or_else(|| input_error(format!("ไม่พบบัญชี “{value}” กรุณาเลือกบัญชีหรือเพิ่มบัญชีก่อนรายการนี้")))
}
fn plan_for(state: &LedgerState, value: &str) -> Result<RecurringExpense, AppError> {
    let found: Vec<_> = state
        .recurring
        .iter()
        .filter(|p| p.id().to_string() == value || p.name().key() == value.trim().to_lowercase())
        .collect();
    if found.len() != 1 {
        return Err(input_error(format!(
            "พบแผนชื่อ “{value}” {} รายการ กรุณาเลือกแผนให้ชัดเจน",
            found.len()
        )));
    }
    Ok(found[0].clone())
}
fn loan_for(state: &LedgerState, value: &str) -> Result<Receivable, AppError> {
    let found: Vec<_> = state
        .receivables
        .iter()
        .filter(|p| {
            p.id().to_string() == value
                || p.debtor().key() == value.trim().to_lowercase()
                || format!("{} · {}", p.debtor().as_str(), p.description().as_str()) == value
        })
        .collect();
    if found.len() != 1 {
        return Err(input_error(format!(
            "พบลูกหนี้ / เรื่องหนี้ “{value}” {} รายการ กรุณาเลือกหนี้ให้ชัดเจน",
            found.len()
        )));
    }
    Ok(found[0].clone())
}
fn entry_for(state: &LedgerState, draft: &PromptDraft) -> Result<JournalEntry, AppError> {
    let reversed: std::collections::BTreeSet<_> = state
        .entries
        .iter()
        .filter_map(|e| match e.kind() {
            EntryKind::Reversal { original } => Some(*original),
            _ => None,
        })
        .collect();
    let amount = if draft.get("amount").is_empty() {
        None
    } else {
        Some(draft.get("amount").parse::<Money>()?)
    };
    let found: Vec<_> = state
        .entries
        .iter()
        .filter(|e| {
            !reversed.contains(&e.id())
                && !matches!(
                    e.kind(),
                    EntryKind::Opening { .. }
                        | EntryKind::ReceivableOpening { .. }
                        | EntryKind::Reversal { .. }
                )
                && (e.id().to_string() == draft.get("entry")
                    || e.note().as_str() == draft.get("entry"))
                && (draft.get("date").is_empty() || e.date().to_string() == draft.get("date"))
                && amount.is_none_or(|a| {
                    e.postings()
                        .iter()
                        .any(|p| p.amount() == a || p.amount() == a.negated())
                })
        })
        .collect();
    if found.len() != 1 {
        return Err(input_error(format!(
            "พบรายการตรงเงื่อนไข {} รายการ กรุณาเลือกรายการที่ต้องการให้แน่นอน",
            found.len()
        )));
    }
    Ok(found[0].clone())
}

/// All mutations here run in a disposable repository. Nothing durable happens.
pub fn prepare_prompt(
    drafts: Vec<PromptDraft>,
    before: LedgerState,
    today: EntryDate,
    ids: &dyn IdSource,
) -> Result<PromptPlan, AppError> {
    if drafts.is_empty() || drafts.len() > MAX_BATCH_ENTRIES {
        return Err(input_error("ตรวจได้ครั้งละ 1–8 รายการ"));
    }
    let repo = PlanningRepository {
        state: before.clone(),
        entries: vec![],
    };
    let mut app = LedgerApplication::new(repo, PromptClock(today), ids);
    let mut descriptions = vec![];
    for (index, mut draft) in drafts.into_iter().enumerate() {
        let state = app.execute(Command::Load)?;
        let Response::Dashboard(view) = state else {
            return Err(input_error("โหลดข้อมูลสำหรับตรวจไม่ได้"));
        };
        let state = state_from_dashboard(&view);
        hydrate_prompt_draft(&mut draft, &state, today);
        let mut impact = String::new();
        let result = (|| -> Result<(), AppError> {
            let questions = draft.questions();
            if !questions.is_empty() {
                return Err(input_error(questions.join(" · ")));
            }
            let command = command_for_draft(&draft, &state, today)?;
            match app.execute(command)? {
                Response::Prepared(p) => {
                    app.execute(Command::Commit(p))?;
                }
                Response::PreparedRecurringPayment(p) => {
                    app.execute(Command::PayRecurring(p))?;
                }
                Response::ReceivableReview(ReceivableReview::New(p)) => {
                    app.execute(Command::CreateReceivable(*p))?;
                }
                Response::ReceivableReview(ReceivableReview::Payment(p)) => {
                    app.execute(Command::ReceiveRepayment(p))?;
                }
                Response::AccountDeletion(p) => {
                    impact = format!(
                        "\nลบถาวร: {} รายการ (รวมยอดเปิดบัญชีและรายการยกเลิก)\n{}",
                        p.entry_ids().len(),
                        p.affected_accounts()
                            .iter()
                            .map(|a| format!(
                                "บัญชี {}: {} → {} บาท",
                                a.account.name().as_str(),
                                a.before,
                                a.after
                            ))
                            .collect::<Vec<_>>()
                            .join("\n")
                    );
                    app.execute(Command::DeleteAccount(p))?;
                }
                Response::Committed(_)
                | Response::CommittedBatch(_)
                | Response::RecurringChanged => {}
                _ => return Err(input_error("คำสั่งนี้ยังไม่มีขั้นตอนบันทึก")),
            }
            Ok(())
        })();
        result.map_err(|e| {
            input_error(format!(
                "รายการที่ {} ({}): {e}",
                index + 1,
                draft.kind.label()
            ))
        })?;
        let details = draft
            .kind
            .fields()
            .into_iter()
            .map(|f| {
                format!(
                    "{}: {}",
                    f.label,
                    prompt_display_value(&state, f.kind, draft.get(f.key))
                )
            })
            .collect::<Vec<_>>()
            .join(" · ");
        descriptions.push(format!(
            "{}. {}\n{details}{impact}",
            index + 1,
            draft.kind.label()
        ));
    }
    let repo = app.into_repository();
    Ok(PromptPlan {
        id: ids.submission_id()?,
        before,
        after: repo.state,
        entries: repo.entries,
        descriptions,
    })
}

pub fn state_from_dashboard(view: &Dashboard) -> LedgerState {
    LedgerState {
        currency: view.currency,
        currency_locked: view.currency_locked,
        thai_tax_enabled: view.thai_tax_enabled,
        accounts: view.accounts.iter().map(|a| a.account.clone()).collect(),
        entries: view.entries.iter().rev().cloned().collect(),
        recurring: view.recurring.clone(),
        settlements: view.settlements.clone(),
        receivables: view.receivables.clone(),
    }
}
pub fn prompt_display_value(state: &LedgerState, kind: PromptFieldType, value: &str) -> String {
    match kind {
        PromptFieldType::Account => state
            .accounts
            .iter()
            .find(|a| a.id().to_string() == value)
            .map(|a| a.name().as_str().to_owned()),
        PromptFieldType::Recurring => state
            .recurring
            .iter()
            .find(|p| p.id().to_string() == value)
            .map(|p| p.name().as_str().to_owned()),
        PromptFieldType::Receivable => state
            .receivables
            .iter()
            .find(|p| p.id().to_string() == value)
            .map(|p| format!("{} · {}", p.debtor().as_str(), p.description().as_str())),
        PromptFieldType::AccountKind => {
            AccountKind::from_code(value).ok().map(|k| k.label().into())
        }
        PromptFieldType::ExpenseCategory | PromptFieldType::IncomeCategory => {
            Category::from_code(value).ok().map(|c| c.label().into())
        }
        PromptFieldType::Entry => state
            .entries
            .iter()
            .find(|e| e.id().to_string() == value)
            .map(|e| format!("{} · {}", e.date(), e.note().as_str())),
        _ => None,
    }
    .unwrap_or_else(|| {
        if value.is_empty() {
            "ไม่กำหนด".into()
        } else {
            value.into()
        }
    })
}
pub fn hydrate_prompt_draft(draft: &mut PromptDraft, state: &LedgerState, today: EntryDate) {
    if draft.kind == PromptKind::PayRecurring
        && let Ok(plan) = plan_for(state, draft.get("plan"))
    {
        if draft.get("amount").is_empty() {
            draft.set("amount", plan.amount().money().to_string());
        }
        if draft.get("account").is_empty()
            && let Some(id) = plan.account()
        {
            draft.set("account", id.to_string());
        }
        if draft.get("month").is_empty()
            && let Ok(view) = dashboard(state.clone(), today)
            && let Some(progress) = recurring_progress(&view)
                .into_iter()
                .find(|p| p.schedule.id() == plan.id())
            && let Some(month) = progress.next_unpaid
        {
            draft.set("month", month.to_string());
        }
    }
}
fn command_for_draft(
    draft: &PromptDraft,
    state: &LedgerState,
    today: EntryDate,
) -> Result<Command, AppError> {
    let s = |key| draft.get(key).to_owned();
    let account = || account_id(state, draft.get("account"));
    let category = |income: bool| -> Result<Category, AppError> {
        (if income {
            &Category::INCOME[..]
        } else {
            &Category::EXPENSE[..]
        })
        .iter()
        .copied()
        .find(|c| c.label() == draft.get("category") || c.code() == draft.get("category"))
        .ok_or_else(|| input_error("กรุณาเลือกหมวดให้ตรงกับชนิดรายการ"))
    };
    Ok(match draft.kind {
        PromptKind::Expense | PromptKind::Income | PromptKind::Transfer => {
            Command::Preview(EntryInput {
                kind: match draft.kind {
                    PromptKind::Income => TransactionKind::Income,
                    PromptKind::Transfer => TransactionKind::Transfer,
                    _ => TransactionKind::Expense,
                },
                amount: s("amount"),
                account: Some(account()?),
                destination: if draft.kind == PromptKind::Transfer {
                    Some(account_id(state, draft.get("destination"))?)
                } else {
                    None
                },
                category: if draft.kind == PromptKind::Transfer {
                    None
                } else {
                    Some(category(draft.kind == PromptKind::Income)?)
                },
                date: s("date"),
                note: s("note"),
                ..EntryInput::empty(today)
            })
        }
        PromptKind::Account => {
            if matches!(draft.get("kind"), "crypto" | "คริปโต" | "Crypto") {
                return Err(input_error("เพิ่มพอร์ตคริปโตจากหน้าบัญชี แล้วระบุจำนวน BTC และ SOL"));
            }
            Command::CreateAccount {
                name: s("name"),
                kind: AccountKind::ALL
                    .into_iter()
                    .find(|k| {
                        k.code() == draft.get("kind")
                            || k.label() == draft.get("kind")
                            || (*k == AccountKind::Bank && draft.get("kind") == "ธนาคาร")
                    })
                    .ok_or_else(|| input_error("กรุณาเลือกประเภทบัญชี"))?,
                opening: s("opening"),
                credit_cycle: if draft.get("closing_day").is_empty()
                    && draft.get("payment_day").is_empty()
                {
                    None
                } else {
                    Some(CreditCardCycle::new(
                        draft
                            .get("closing_day")
                            .parse()
                            .map_err(|_| DomainError::InvalidCreditCycle)?,
                        draft
                            .get("payment_day")
                            .parse()
                            .map_err(|_| DomainError::InvalidCreditCycle)?,
                    )?)
                },
            }
        }
        PromptKind::Recurring => Command::AddRecurring(RecurringInput {
            name: s("name"),
            amount: s("amount"),
            day: s("day"),
            start: s("start"),
            account: Some(account()?),
            category: Some(category(false)?),
            installments: optional_count(draft)?,
        }),
        PromptKind::CountRecurring => Command::SetRecurringInstallments {
            expected: plan_for(state, draft.get("plan"))?,
            installments: optional_count(draft)?,
        },
        PromptKind::StopRecurring => Command::StopRecurring {
            expected: plan_for(state, draft.get("plan"))?,
            month: draft.get("month").parse()?,
        },
        PromptKind::PayRecurring => {
            let plan = plan_for(state, draft.get("plan"))?;
            let mut input = recurring_payment_input(&plan, today);
            input.amount = s("amount");
            input.account = Some(account()?);
            input.date = s("date");
            Command::PreviewRecurringPayment {
                expected: plan,
                month: draft.get("month").parse()?,
                input,
            }
        }
        PromptKind::LinkRecurring => Command::LinkRecurring {
            expected: plan_for(state, draft.get("plan"))?,
            month: draft.get("month").parse()?,
            entry: entry_for(state, draft)?.id(),
        },
        PromptKind::Receivable | PromptKind::Lending => {
            Command::PreviewReceivable(ReceivableInput {
                debtor: s("debtor"),
                description: s("description"),
                total: s("amount"),
                opened: s("date"),
                start: s("start"),
                day: if draft.get("day").is_empty() || draft.get("day") == "ไม่กำหนด"
                {
                    None
                } else {
                    Some(s("day"))
                },
                installments: optional_count(draft)?,
                source: if draft.kind == PromptKind::Lending {
                    Some(account()?)
                } else {
                    None
                },
            })
        }
        PromptKind::Repayment => Command::PreviewRepayment(RepaymentInput {
            receivable: loan_for(state, draft.get("loan"))?.id(),
            account: Some(account()?),
            principal: s("amount"),
            interest: s("interest"),
            date: s("date"),
        }),
        PromptKind::Reverse => Command::Reverse(entry_for(state, draft)?.id()),
        PromptKind::DeleteAccount => Command::PreviewDeleteAccount(account()?),
    })
}
struct PromptClock(EntryDate);
impl Clock for PromptClock {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok(self.0)
    }
}
struct PlanningRepository {
    state: LedgerState,
    entries: Vec<PreparedEntry>,
}
impl PlanningRepository {
    fn append(&mut self, p: PreparedEntry) -> Result<CommitOutcome, StorageError> {
        validate_append(&self.state, &p.entry)?;
        if let Some(link) = &p.recurring {
            validate_recurring_settlement(&self.state, &link.schedule, link.month, &p.entry)?;
            self.state
                .settlements
                .retain(|s| !(s.recurring == link.schedule.id() && s.month == link.month));
            self.state.settlements.push(RecurringSettlement {
                recurring: link.schedule.id(),
                month: link.month,
                entry: p.entry.id(),
            });
        }
        let id = p.entry.id();
        self.state.entries.push(p.entry.clone());
        self.entries.push(p);
        Ok(CommitOutcome::Saved(id))
    }
}
impl LedgerRepository for PlanningRepository {
    fn set_crypto_holdings(&mut self, _: &Account, _: CryptoHoldings) -> Result<(), StorageError> {
        Err(StorageError::Corrupt)
    }
    fn crypto_prices(&mut self) -> Result<Option<CryptoPrices>, StorageError> {
        Ok(None)
    }
    fn save_crypto_prices(&mut self, _: CryptoPrices) -> Result<(), StorageError> {
        Err(StorageError::Corrupt)
    }
    fn set_credit_cycle(
        &mut self,
        expected: &Account,
        cycle: CreditCardCycle,
    ) -> Result<(), StorageError> {
        let updated = configure_credit_cycle(&self.state, expected, cycle)?;
        if let Some(account) = self
            .state
            .accounts
            .iter_mut()
            .find(|a| a.id() == expected.id())
        {
            *account = updated;
        }
        Ok(())
    }
    fn preferences(&mut self) -> Result<UserPreferences, StorageError> {
        Err(StorageError::Corrupt)
    }
    fn set_preferences(&mut self, _: UserPreferences) -> Result<(), StorageError> {
        Err(StorageError::Corrupt)
    }
    fn set_thai_tax_enabled(&mut self, enabled: bool) -> Result<(), StorageError> {
        if enabled && self.state.currency != Currency::Thb {
            return Err(DomainError::InvalidCurrency.into());
        }
        self.state.thai_tax_enabled = enabled;
        Ok(())
    }
    fn set_currency(&mut self, currency: Currency) -> Result<(), StorageError> {
        if self.state.currency_locked && self.state.currency != currency {
            return Err(StorageError::CurrencyLocked);
        }
        self.state.currency = currency;
        if currency != Currency::Thb {
            self.state.thai_tax_enabled = false;
        }
        Ok(())
    }
    fn commit_prompt(&mut self, _: &PromptPlan) -> Result<(), StorageError> {
        Err(StorageError::Corrupt)
    }
    fn snapshot(&mut self) -> Result<LedgerState, StorageError> {
        Ok(self.state.clone())
    }
    fn commit(
        &mut self,
        entry: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError> {
        self.append(PreparedEntry {
            entry: entry.clone(),
            submission,
            recurring: None,
        })
    }
    fn commit_batch(
        &mut self,
        entries: &[PreparedEntry],
    ) -> Result<Vec<CommitOutcome>, StorageError> {
        entries.iter().cloned().map(|p| self.append(p)).collect()
    }
    fn create_account(
        &mut self,
        account: &Account,
        opening: &JournalEntry,
        submission: SubmissionId,
    ) -> Result<CommitOutcome, StorageError> {
        account.ensure_can_add(&self.state.accounts)?;
        self.state.currency_locked = true;
        self.state.accounts.push(account.clone());
        self.commit(opening, submission)
    }
    fn create_receivable(&mut self, p: &PreparedReceivable) -> Result<CommitOutcome, StorageError> {
        self.state.currency_locked = true;
        self.state.receivables.push(p.loan.clone());
        self.append(p.opening.clone())
    }
    fn replace_recurring(
        &mut self,
        expected: &RecurringExpense,
        replacement: &RecurringExpense,
    ) -> Result<(), StorageError> {
        let stopped = revised_recurring(&self.state, expected, replacement)?;
        for schedule in &mut self.state.recurring {
            if schedule.id() == expected.id() {
                *schedule = stopped.clone();
            }
        }
        self.state.recurring.push(replacement.clone());
        Ok(())
    }
    fn add_recurring(&mut self, schedule: &RecurringExpense) -> Result<(), StorageError> {
        validate_new_recurring(&self.state, schedule)?;
        self.state.currency_locked = true;
        self.state.recurring.push(schedule.clone());
        Ok(())
    }
    fn set_recurring_installments(
        &mut self,
        expected: &RecurringExpense,
        count: Option<u32>,
    ) -> Result<(), StorageError> {
        let changed = changed_installments(&self.state, expected, count)?;
        if let Some(s) = self
            .state
            .recurring
            .iter_mut()
            .find(|s| s.id() == expected.id())
        {
            *s = changed;
        }
        Ok(())
    }
    fn stop_recurring(
        &mut self,
        expected: &RecurringExpense,
        month: Month,
    ) -> Result<(), StorageError> {
        if !self.state.recurring.contains(expected) || expected.stopped_from().is_some() {
            return Err(StorageError::RecurringChanged);
        }
        let changed = expected.clone().stop_from(month)?;
        if let Some(s) = self
            .state
            .recurring
            .iter_mut()
            .find(|s| s.id() == expected.id())
        {
            *s = changed;
        }
        Ok(())
    }
    fn pay_recurring(
        &mut self,
        p: &PreparedRecurringPayment,
    ) -> Result<CommitOutcome, StorageError> {
        let mut entry = p.payment.clone();
        entry.recurring = Some(PreparedRecurringLink {
            schedule: p.schedule.clone(),
            month: p.month,
        });
        self.append(entry)
    }
    fn link_recurring(
        &mut self,
        expected: &RecurringExpense,
        month: Month,
        entry: EntryId,
    ) -> Result<(), StorageError> {
        let journal = self
            .state
            .entries
            .iter()
            .find(|e| e.id() == entry)
            .ok_or(StorageError::RecurringChanged)?;
        validate_recurring_settlement(&self.state, expected, month, journal)?;
        self.state
            .settlements
            .retain(|s| !(s.recurring == expected.id() && s.month == month));
        self.state.settlements.push(RecurringSettlement {
            recurring: expected.id(),
            month,
            entry,
        });
        Ok(())
    }
    fn delete_account(&mut self, deletion: &AccountDeletion) -> Result<(), StorageError> {
        deletion.validate_current(&self.state)?;
        self.state
            .accounts
            .retain(|a| a.id() != deletion.account().id());
        self.state
            .entries
            .retain(|e| !deletion.entry_ids().contains(&e.id()));
        self.entries
            .retain(|e| !deletion.entry_ids().contains(&e.entry.id()));
        self.state
            .settlements
            .retain(|s| !deletion.entry_ids().contains(&s.entry));
        self.state.recurring = self
            .state
            .recurring
            .iter()
            .map(|s| {
                if s.account() == Some(deletion.account().id()) {
                    s.clone().without_account()
                } else {
                    s.clone()
                }
            })
            .collect();
        Ok(())
    }
}
