use crate::Dashboard;
use chrono::{Days, NaiveDate};
use ledger_domain::{DomainError, EntryKind, Money};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyExpense {
    pub date: NaiveDate,
    pub amount: Money,
    pub count: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySpending {
    pub days: Vec<DailyExpense>,
    pub total: Money,
}
/// Seven calendar days ending today, including empty days and crossing months/years.
/// Transfers, openings, income, cancelled expenses and unpaid plans are not spending.
pub fn daily_spending(view: &Dashboard) -> Result<DailySpending, DomainError> {
    let today = view.today.date();
    let mut days = (0..7)
        .rev()
        .map(|offset| {
            Ok(DailyExpense {
                date: today
                    .checked_sub_days(Days::new(offset))
                    .ok_or(DomainError::InvalidDate)?,
                amount: Money::ZERO,
                count: 0,
            })
        })
        .collect::<Result<Vec<_>, DomainError>>()?;
    let mut total = Money::ZERO;
    for entry in &view.entries {
        if view.reversed.contains(&entry.id()) {
            continue;
        }
        let EntryKind::Expense { amount, .. } = entry.kind() else {
            continue;
        };
        if let Some(day) = days.iter_mut().find(|day| day.date == entry.date().date()) {
            day.amount = day.amount.checked_add(amount.money())?;
            day.count += 1;
            total = total.checked_add(amount.money())?;
        }
    }
    Ok(DailySpending { days, total })
}
