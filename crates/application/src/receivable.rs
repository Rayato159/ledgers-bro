use crate::*;
use ledger_domain::*;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_RECEIVABLES: usize = 500;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivableInput {
    pub debtor: String,
    pub description: String,
    pub total: String,
    pub opened: String,
    pub start: String,
    pub day: Option<String>,
    pub installments: Option<String>,
    /// None registers a debt already owed; Some records money lent now.
    pub source: Option<AccountId>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedReceivable {
    pub loan: Receivable,
    pub opening: PreparedEntry,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepaymentInput {
    pub receivable: ReceivableId,
    pub account: Option<AccountId>,
    pub principal: String,
    pub interest: String,
    pub date: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceivableReview {
    New(Box<PreparedReceivable>),
    Payment(Vec<PreparedEntry>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceivableStatus {
    Collecting,
    Overdue,
    Unscheduled,
    Paid,
    Cancelled,
}
impl ReceivableStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Collecting => "กำลังผ่อนชำระ",
            Self::Overdue => "มีงวดเลยกำหนด",
            Self::Unscheduled => "ไม่กำหนดแผนยอดรายงวด",
            Self::Paid => "ชำระครบแล้ว",
            Self::Cancelled => "ยกเลิกการให้ยืมแล้ว",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivableProgress {
    pub loan: Receivable,
    pub paid: Money,
    pub outstanding: Money,
    pub overdue: Money,
    pub paid_installments: Option<u32>,
    pub remaining_installments: Option<u32>,
    pub next_due: Option<EntryDate>,
    pub next_amount: Option<Money>,
    pub status: ReceivableStatus,
    pub payments: Vec<JournalEntry>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivableSummary {
    pub items: Vec<ReceivableProgress>,
    pub total: Money,
    pub paid: Money,
    pub outstanding: Money,
    pub overdue: Money,
}

/// Check cross-aggregate facts both at preview and inside the repository lock.
pub fn validate_receivables(state: &LedgerState) -> Result<(), StorageError> {
    if state.receivables.len() > MAX_RECEIVABLES {
        return Err(StorageError::ReceivableLimit);
    }
    let ids: BTreeSet<_> = state.receivables.iter().map(Receivable::id).collect();
    if ids.len() != state.receivables.len() {
        return Err(StorageError::Corrupt);
    }
    let reversed: BTreeSet<_> = state
        .entries
        .iter()
        .filter_map(|e| match e.kind() {
            EntryKind::Reversal { original } => Some(*original),
            _ => None,
        })
        .collect();
    let mut origins = BTreeMap::new();
    let mut total_principal = 0_i128;
    let mut balances: BTreeMap<ReceivableId, i128> = BTreeMap::new();
    for entry in &state.entries {
        let (id, delta, origin) = match entry.kind() {
            EntryKind::ReceivableOpening { receivable, amount }
            | EntryKind::Lending {
                receivable, amount, ..
            } => (*receivable, amount.money().minor(), true),
            EntryKind::Repayment {
                receivable, amount, ..
            } => (*receivable, -amount.money().minor(), false),
            _ => continue,
        };
        let loan = state
            .receivables
            .iter()
            .find(|r| r.id() == id)
            .ok_or(StorageError::ReceivableChanged)?;
        if entry.date() < loan.opened() {
            return Err(StorageError::ReceivableDate);
        }
        if origin
            && (origins.insert(id, entry.id()).is_some()
                || delta != loan.total().money().minor()
                || entry.date() != loan.opened())
        {
            return Err(StorageError::Corrupt);
        }
        if !reversed.contains(&entry.id()) {
            if origin {
                total_principal += i128::from(delta);
            }
            *balances.entry(id).or_default() += i128::from(delta);
        }
    }
    checked_total(total_principal)?;
    for loan in &state.receivables {
        if !origins.contains_key(&loan.id()) {
            return Err(StorageError::Corrupt);
        }
        let balance = *balances.get(&loan.id()).unwrap_or(&0);
        if balance < 0 || balance > i128::from(loan.total().money().minor()) {
            return Err(StorageError::ReceivableOverpayment);
        }
    }
    Ok(())
}

fn checked_total(value: i128) -> Result<Money, DomainError> {
    Money::from_minor(i64::try_from(value).map_err(|_| DomainError::MoneyOverflow)?)
}

pub fn receivable_summary(view: &Dashboard) -> Result<ReceivableSummary, DomainError> {
    let mut result = ReceivableSummary {
        items: vec![],
        total: Money::ZERO,
        paid: Money::ZERO,
        outstanding: Money::ZERO,
        overdue: Money::ZERO,
    };
    let mut totals = [0_i128; 4];
    for loan in &view.receivables {
        if loan.opened() > view.today {
            continue;
        }
        let cancelled = view.entries.iter().any(|e| {
            matches!(e.kind(), EntryKind::Lending { receivable, .. } if *receivable == loan.id())
                && view.reversed.contains(&e.id())
        });
        let payments: Vec<_> = view.entries.iter().filter(|e| e.date() <= view.today && !view.reversed.contains(&e.id()) && matches!(e.kind(), EntryKind::Repayment { receivable, .. } if *receivable == loan.id())).cloned().collect();
        let paid = payments
            .iter()
            .try_fold(Money::ZERO, |sum, e| match e.kind() {
                EntryKind::Repayment { amount, .. } => sum.checked_add(amount.money()),
                _ => Ok(sum),
            })?;
        let total = if cancelled {
            Money::ZERO
        } else {
            loan.total().money()
        };
        let outstanding = total.checked_sub(paid)?;
        let mut completed = 0;
        let mut cumulative = Money::ZERO;
        let mut due_total = Money::ZERO;
        let mut next_due = None;
        let mut next_amount = None;
        if !cancelled && let Some(count) = loan.installments() {
            for n in 1..=count {
                cumulative = cumulative.checked_add(loan.installment_amount(n)?)?;
                let due = loan
                    .day()
                    .map(|day| loan.start().shifted(n as i32 - 1)?.on_day(day))
                    .transpose()?;
                if cumulative <= paid {
                    completed += 1;
                } else if next_amount.is_none() {
                    next_due = due;
                    next_amount = Some(cumulative.checked_sub(paid)?);
                }
                if due.is_some_and(|date| date < view.today) {
                    due_total = cumulative;
                }
            }
        } else if !cancelled
            && outstanding > Money::ZERO
            && let Some(day) = loan.day()
        {
            let last = payments.iter().map(JournalEntry::date).max();
            let month = match last {
                Some(date) => Month::of(date)?
                    .shifted(1)
                    .ok()
                    .map(|m| m.max(loan.start())),
                None => Some(loan.start()),
            };
            next_due = month.map(|m| m.on_day(day)).transpose()?;
        }
        let overdue = Money::from_minor((due_total.minor() - paid.minor()).max(0))?;
        let status = if cancelled {
            ReceivableStatus::Cancelled
        } else if outstanding == Money::ZERO {
            ReceivableStatus::Paid
        } else if overdue > Money::ZERO {
            ReceivableStatus::Overdue
        } else if loan.day().is_none() || loan.installments().is_none() {
            ReceivableStatus::Unscheduled
        } else {
            ReceivableStatus::Collecting
        };
        for (sum, value) in totals.iter_mut().zip([total, paid, outstanding, overdue]) {
            *sum += i128::from(value.minor());
        }
        result.items.push(ReceivableProgress {
            loan: loan.clone(),
            paid,
            outstanding,
            overdue,
            paid_installments: loan.installments().map(|_| completed),
            remaining_installments: loan
                .installments()
                .map(|n| if cancelled { 0 } else { n - completed }),
            next_due,
            next_amount,
            status,
            payments,
        });
    }
    [
        result.total,
        result.paid,
        result.outstanding,
        result.overdue,
    ] = [
        checked_total(totals[0])?,
        checked_total(totals[1])?,
        checked_total(totals[2])?,
        checked_total(totals[3])?,
    ];
    Ok(result)
}
