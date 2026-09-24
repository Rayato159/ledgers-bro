use crate::Dashboard;
use ledger_domain::{DomainError, EntryKind, Money};

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
