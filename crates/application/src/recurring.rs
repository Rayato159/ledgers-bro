use crate::{
    AppError, Dashboard, EntryInput, LedgerState, MonthlyFlow, PreparedEntry, StorageError,
};
use ledger_domain::*;

pub const MAX_RECURRING: usize = 500;
pub fn validate_recurring_totals(schedules: &[RecurringExpense]) -> Result<(), StorageError> {
    for month in schedules.iter().map(|s| s.due().start()) {
        let sum: i128 = schedules
            .iter()
            .filter(|s| s.occurs_in(month))
            .map(|s| i128::from(s.amount().money().minor()))
            .sum();
        Money::from_minor(i64::try_from(sum).map_err(|_| DomainError::MoneyOverflow)?)?;
    }
    Ok(())
}
pub fn validate_new_recurring(
    state: &LedgerState,
    schedule: &RecurringExpense,
) -> Result<(), StorageError> {
    if state.recurring.len() >= MAX_RECURRING {
        return Err(StorageError::RecurringLimit);
    }
    if !state
        .accounts
        .iter()
        .any(|a| Some(a.id()) == schedule.account() && !a.is_archived())
    {
        return Err(DomainError::AccountUnavailable.into());
    }
    let mut schedules = state.recurring.clone();
    schedules.push(schedule.clone());
    validate_recurring_totals(&schedules)
}
pub fn changed_installments(
    state: &LedgerState,
    expected: &RecurringExpense,
    count: Option<u32>,
) -> Result<RecurringExpense, StorageError> {
    if !state.recurring.contains(expected) {
        return Err(StorageError::RecurringChanged);
    }
    let changed = expected.clone().with_installments(count)?;
    if state
        .settlements
        .iter()
        .any(|s| s.recurring == expected.id() && changed.due().number_in(s.month).is_none())
    {
        return Err(StorageError::InstallmentsConflict);
    }
    let schedules: Vec<_> = state
        .recurring
        .iter()
        .map(|s| {
            if s.id() == expected.id() {
                changed.clone()
            } else {
                s.clone()
            }
        })
        .collect();
    validate_recurring_totals(&schedules)?;
    Ok(changed)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringInput {
    pub name: String,
    pub amount: String,
    pub day: String,
    pub start: String,
    pub account: Option<AccountId>,
    pub category: Option<Category>,
    pub installments: Option<String>,
}
impl RecurringInput {
    pub fn validate(self, id: RecurringId) -> Result<RecurringExpense, AppError> {
        Ok(RecurringExpense::new(
            id,
            AccountName::new(&self.name)?,
            PositiveMoney::new(self.amount.parse()?)?,
            self.category.ok_or(DomainError::InvalidCategory)?,
            self.account,
            MonthlyDue::new(
                self.day
                    .parse()
                    .map_err(|_| DomainError::InvalidRecurring)?,
                self.start.parse()?,
            )?
            .with_installments(parse_installments(self.installments.as_deref())?)?,
        )?)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringSettlement {
    pub recurring: RecurringId,
    pub month: Month,
    pub entry: EntryId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRecurringPayment {
    pub schedule: RecurringExpense,
    pub month: Month,
    pub payment: PreparedEntry,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringOccurrence {
    pub schedule: RecurringExpense,
    pub due: EntryDate,
    pub paid_entry: Option<EntryId>,
    pub paid_amount: Money,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringMonth {
    pub items: Vec<RecurringOccurrence>,
    pub planned: Money,
    pub paid: Money,
    pub pending: Money,
}

pub fn recurring_month(view: &Dashboard, month: Month) -> Result<RecurringMonth, DomainError> {
    let mut result = RecurringMonth {
        items: vec![],
        planned: Money::ZERO,
        paid: Money::ZERO,
        pending: Money::ZERO,
    };
    for schedule in &view.recurring {
        let paid = view
            .settlements
            .iter()
            .find(|s| s.recurring == schedule.id() && s.month == month)
            .and_then(|s| {
                view.entries.iter().find(|e| {
                    e.id() == s.entry && e.date() <= view.today && !view.reversed.contains(&e.id())
                })
            });
        if !schedule.occurs_in(month) && paid.is_none() {
            continue;
        }
        let paid_amount = match paid.map(JournalEntry::kind) {
            Some(EntryKind::Expense { amount, .. }) => amount.money(),
            _ => Money::ZERO,
        };
        result.planned = result.planned.checked_add(schedule.amount().money())?;
        result.paid = result.paid.checked_add(paid_amount)?;
        if paid.is_none() {
            result.pending = result.pending.checked_add(schedule.amount().money())?;
        }
        result.items.push(RecurringOccurrence {
            schedule: schedule.clone(),
            due: month.on_day(schedule.due().day())?,
            paid_entry: paid.map(JournalEntry::id),
            paid_amount,
        });
    }
    result
        .items
        .sort_by_key(|item| (item.due, item.schedule.name().key(), item.schedule.id()));
    Ok(result)
}

/// A projection combines recorded expenses with still-unpaid obligations of the
/// same month. Linked payments (including early payments) are never added twice.
pub fn projected_cashflow(
    view: &Dashboard,
    actual: &MonthlyFlow,
) -> Result<MonthlyFlow, DomainError> {
    let pending = recurring_month(view, Month::new(actual.year, actual.month)?)?.pending;
    Ok(MonthlyFlow {
        expenses: actual.expenses.checked_add(pending)?,
        ..actual.clone()
    })
}

pub fn recurring_payment_input(schedule: &RecurringExpense, today: EntryDate) -> EntryInput {
    EntryInput {
        amount: schedule.amount().money().to_string(),
        account: schedule.account(),
        category: Some(schedule.category()),
        note: schedule.name().as_str().to_owned(),
        ..EntryInput::empty(today)
    }
}

/// Shared policy, rechecked against the locked repository snapshot.
pub fn validate_recurring_settlement(
    state: &LedgerState,
    expected: &RecurringExpense,
    month: Month,
    entry: &JournalEntry,
) -> Result<(), StorageError> {
    if !state.recurring.contains(expected) || !expected.occurs_in(month) {
        return Err(StorageError::RecurringChanged);
    }
    if !matches!(entry.kind(), EntryKind::Expense { .. }) {
        return Err(DomainError::InvalidCategory.into());
    }
    let reversed = |id| {
        state
            .entries
            .iter()
            .any(|e| matches!(e.kind(), EntryKind::Reversal { original } if *original == id))
    };
    if reversed(entry.id()) {
        return Err(DomainError::InvalidReversal.into());
    }
    if state.settlements.iter().any(|s| {
        s.entry == entry.id()
            || (s.recurring == expected.id() && s.month == month && !reversed(s.entry))
    }) {
        return Err(StorageError::RecurringPaid);
    }
    Ok(())
}

pub fn parse_installments(text: Option<&str>) -> Result<Option<u32>, DomainError> {
    text.map(|s| {
        s.trim()
            .parse::<u32>()
            .map_err(|_| DomainError::InvalidInstallments)
            .and_then(|n| {
                if (1..=MonthlyDue::MAX_INSTALLMENTS).contains(&n) {
                    Ok(n)
                } else {
                    Err(DomainError::InvalidInstallments)
                }
            })
    })
    .transpose()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringSelection {
    pub recurring: RecurringId,
    pub month: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedRecurringLink {
    pub schedule: RecurringExpense,
    pub month: Month,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringProgress {
    pub schedule: RecurringExpense,
    pub paid: u32,
    pub remaining: Option<u32>,
    pub completed: bool,
    pub next_unpaid: Option<Month>,
}

/// Completion is derived from uncancelled, dated payments. Reversing or deleting
/// a payment reopens its original installment instead of inventing another month.
pub fn recurring_progress(view: &Dashboard) -> Vec<RecurringProgress> {
    use std::collections::{BTreeMap, BTreeSet};
    let valid_entries: BTreeSet<_> = view
        .entries
        .iter()
        .filter(|e| e.date() <= view.today && !view.reversed.contains(&e.id()))
        .map(JournalEntry::id)
        .collect();
    let mut paid_months: BTreeMap<RecurringId, BTreeSet<Month>> = BTreeMap::new();
    for s in &view.settlements {
        if valid_entries.contains(&s.entry) {
            paid_months.entry(s.recurring).or_default().insert(s.month);
        }
    }
    view.recurring
        .iter()
        .map(|schedule| {
            let paid = paid_months
                .get(&schedule.id())
                .map_or(0, |months| months.len() as u32);
            let remaining = schedule
                .due()
                .installments()
                .map(|n| n.saturating_sub(paid));
            // At most paid+1 checks: the first gap must occur by then, even for an
            // unlimited plan. Do not enumerate an unbounded future in the picker.
            let next_unpaid = (0..=paid).find_map(|offset| {
                let month = schedule.due().start().shifted(offset as i32).ok()?;
                (schedule.occurs_in(month)
                    && !paid_months
                        .get(&schedule.id())
                        .is_some_and(|months| months.contains(&month)))
                .then_some(month)
            });
            RecurringProgress {
                schedule: schedule.clone(),
                paid,
                remaining,
                completed: remaining == Some(0),
                next_unpaid,
            }
        })
        .collect()
}

/// Explicitly selecting a plan fills only blanks. Preserve amounts, dates and
/// accounts supplied in the prompt/manual form for the user to review.
pub fn select_recurring(
    input: &EntryInput,
    schedule: &RecurringExpense,
    month: Month,
) -> EntryInput {
    let mut result = input.clone();
    result.recurring = Some(RecurringSelection {
        recurring: schedule.id(),
        month: month.to_string(),
    });
    if result.amount.trim().is_empty() {
        result.amount = schedule.amount().money().to_string();
    }
    if result.account.is_none() {
        result.account = schedule.account();
    }
    if result.category.is_none() {
        result.category = Some(schedule.category());
    }
    if result.note.trim().is_empty() {
        result.note = schedule.name().as_str().to_owned();
    }
    result
}
