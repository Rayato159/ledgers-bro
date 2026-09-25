use crate::Dashboard;
use ledger_domain::{Category, DomainError, EntryKind, JournalEntry, Money, Month};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpenseSlice {
    pub category: Category,
    pub description: Option<String>,
    pub amount: Money,
}

pub fn expense_description(entry: &JournalEntry) -> String {
    entry
        .note()
        .as_str()
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_owned()
}

/// Keep other expenses identifiable instead of merging unrelated purchases.
pub fn expense_slices(view: &Dashboard, month: Month) -> Result<Vec<ExpenseSlice>, DomainError> {
    let mut groups = std::collections::BTreeMap::new();
    for entry in &view.entries {
        if entry.date() > view.today
            || entry.date().month_key() != month.key()
            || view.reversed.contains(&entry.id())
        {
            continue;
        }
        if let EntryKind::Expense {
            category, amount, ..
        } = entry.kind()
        {
            let description =
                (*category == Category::OtherExpense).then(|| expense_description(entry));
            let slice = groups
                .entry((category.code(), description.clone()))
                .or_insert(ExpenseSlice {
                    category: *category,
                    description,
                    amount: Money::ZERO,
                });
            slice.amount = slice.amount.checked_add(amount.money())?;
        }
    }
    let mut slices: Vec<_> = groups.into_values().collect();
    slices.sort_by(|a, b| {
        b.amount
            .minor()
            .cmp(&a.amount.minor())
            .then(a.category.code().cmp(b.category.code()))
            .then(a.description.cmp(&b.description))
    });
    Ok(slices)
}

/// Product indicator for recorded income/expenses, not a credit or solvency score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowHealth {
    NoData,
    Poor,
    Fair,
    Good,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyFlow {
    pub year: i32,
    pub month: u32,
    pub income: Money,
    pub expenses: Money,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CashflowBreakdown {
    pub recorded: MonthlyFlow,
    pub income_entries: Vec<JournalEntry>,
    pub expense_entries: Vec<JournalEntry>,
    pub pending_items: Vec<crate::RecurringOccurrence>,
    pub pending_total: Money,
}

/// Drill-down uses the same effective-date and reversal rules as the chart.
/// Pending plans stay separate: they have not moved cash or created an expense.
pub fn cashflow_breakdown(
    view: &Dashboard,
    month: Month,
) -> Result<CashflowBreakdown, DomainError> {
    let (year, m) = month.key();
    let mut result = CashflowBreakdown {
        recorded: MonthlyFlow {
            year,
            month: m,
            income: Money::ZERO,
            expenses: Money::ZERO,
        },
        income_entries: Vec::new(),
        expense_entries: Vec::new(),
        pending_items: Vec::new(),
        pending_total: Money::ZERO,
    };
    for entry in &view.entries {
        if entry.date() > view.today
            || entry.date().month_key() != month.key()
            || view.reversed.contains(&entry.id())
        {
            continue;
        }
        match entry.kind() {
            EntryKind::Income { amount, .. } => {
                result.recorded.income = result.recorded.income.checked_add(amount.money())?;
                result.income_entries.push(entry.clone());
            }
            EntryKind::Expense { amount, .. } => {
                result.recorded.expenses = result.recorded.expenses.checked_add(amount.money())?;
                result.expense_entries.push(entry.clone());
            }
            _ => {}
        }
    }
    let recurring = crate::recurring_month(view, month)?;
    result.pending_total = recurring.pending;
    result.pending_items = recurring
        .items
        .into_iter()
        .filter(|i| i.paid_entry.is_none())
        .collect();
    Ok(result)
}

impl MonthlyFlow {
    fn total(&self) -> i128 {
        i128::from(self.income.minor()) + i128::from(self.expenses.minor())
    }

    /// Signed ten-thousandths in [-10000, 10000]. None means no recorded activity.
    pub fn rate(&self) -> Option<i32> {
        let total = self.total();
        (total > 0).then(|| {
            let difference = i128::from(self.income.minor()) - i128::from(self.expenses.minor());
            (difference * 10_000 / total) as i32
        })
    }

    pub fn income_share(&self) -> Option<u32> {
        let total = self.total();
        (total > 0).then(|| ((i128::from(self.income.minor()) * 10_000 + total / 2) / total) as u32)
    }

    /// Percentage difference relative to the smaller side, in hundredths of a percent.
    /// Undefined when that side is zero; do not display infinity or a made-up 100%.
    pub fn difference_percent(&self) -> Option<u64> {
        let smaller = self.income.minor().min(self.expenses.minor());
        (smaller > 0).then(|| {
            let difference =
                (i128::from(self.income.minor()) - i128::from(self.expenses.minor())).abs();
            (difference * 10_000 / i128::from(smaller)) as u64
        })
    }

    pub fn health(&self) -> FlowHealth {
        let total = self.total();
        let difference = i128::from(self.income.minor()) - i128::from(self.expenses.minor());
        if total == 0 {
            FlowHealth::NoData
        } else if difference < 0 {
            FlowHealth::Poor
        } else if difference * 10 < total {
            FlowHealth::Fair
        } else {
            FlowHealth::Good
        }
    }
}

/// Six calendar months, oldest first. Transfers, openings and cancelled entries
/// never contribute. A reversal removes the original from its effective month.
pub fn monthly_cashflow(view: &Dashboard) -> Result<Vec<MonthlyFlow>, DomainError> {
    let (year, month) = view.today.month_key();
    let current = year * 12 + month as i32 - 1;
    let mut months: Vec<_> = (0..6)
        .rev()
        .map(|offset| {
            let index = current - offset;
            MonthlyFlow {
                year: index.div_euclid(12),
                month: index.rem_euclid(12) as u32 + 1,
                income: Money::ZERO,
                expenses: Money::ZERO,
            }
        })
        .collect();
    for entry in &view.entries {
        if entry.date() > view.today || view.reversed.contains(&entry.id()) {
            continue;
        }
        let Some(period) = months
            .iter_mut()
            .find(|period| (period.year, period.month) == entry.date().month_key())
        else {
            continue;
        };
        match entry.kind() {
            EntryKind::Income { amount, .. } => {
                period.income = period.income.checked_add(amount.money())?;
            }
            EntryKind::Expense { amount, .. } => {
                period.expenses = period.expenses.checked_add(amount.money())?;
            }
            _ => {}
        }
    }
    Ok(months)
}
