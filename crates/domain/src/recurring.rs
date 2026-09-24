use crate::{AccountId, AccountName, Category, DomainError, EntryDate, PositiveMoney, RecurringId};
use chrono::NaiveDate;
use std::{fmt, str::FromStr};

/// Calendar month independent of the clock; recurrence never adds 30 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Month(EntryDate);
impl Month {
    pub fn new(year: i32, month: u32) -> Result<Self, DomainError> {
        Ok(Self(EntryDate::new(
            NaiveDate::from_ymd_opt(year, month, 1).ok_or(DomainError::InvalidDate)?,
        )?))
    }
    pub fn of(date: EntryDate) -> Result<Self, DomainError> {
        let (year, month) = date.month_key();
        Self::new(year, month)
    }
    pub fn key(self) -> (i32, u32) {
        self.0.month_key()
    }
    pub fn shifted(self, offset: i32) -> Result<Self, DomainError> {
        let (year, month) = self.key();
        let index = (year * 12 + month as i32 - 1)
            .checked_add(offset)
            .ok_or(DomainError::InvalidDate)?;
        Self::new(index.div_euclid(12), index.rem_euclid(12) as u32 + 1)
    }
    pub fn on_day(self, day: u32) -> Result<EntryDate, DomainError> {
        if !(1..=31).contains(&day) {
            return Err(DomainError::InvalidRecurring);
        }
        let (year, month) = self.key();
        let date = (1..=day)
            .rev()
            .find_map(|d| NaiveDate::from_ymd_opt(year, month, d))
            .ok_or(DomainError::InvalidDate)?;
        EntryDate::new(date)
    }
}
impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month) = self.key();
        write!(f, "{year:04}-{month:02}")
    }
}
impl FromStr for Month {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 7 {
            return Err(DomainError::InvalidDate);
        }
        Self::of(format!("{value}-01").parse()?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthlyDue {
    day: u32,
    start: Month,
    installments: Option<u32>,
}
impl MonthlyDue {
    pub fn new(day: u32, start: Month) -> Result<Self, DomainError> {
        start.on_day(day)?;
        Ok(Self {
            day,
            start,
            installments: None,
        })
    }
    pub const MAX_INSTALLMENTS: u32 = 1200;
    pub fn with_installments(mut self, count: Option<u32>) -> Result<Self, DomainError> {
        if let Some(count) = count {
            if !(1..=Self::MAX_INSTALLMENTS).contains(&count) {
                return Err(DomainError::InvalidInstallments);
            }
            self.start.shifted(count as i32 - 1)?;
        }
        self.installments = count;
        Ok(self)
    }
    pub fn installments(self) -> Option<u32> {
        self.installments
    }
    /// One-based contractual installment, independent of when it was paid.
    pub fn number_in(self, month: Month) -> Option<u32> {
        let (year, m) = month.key();
        let (start_year, start_m) = self.start.key();
        let offset = (year - start_year) * 12 + m as i32 - start_m as i32;
        if offset < 0
            || self
                .installments
                .is_some_and(|count| offset as u32 >= count)
        {
            None
        } else {
            Some(offset as u32 + 1)
        }
    }
    pub fn day(self) -> u32 {
        self.day
    }
    pub fn start(self) -> Month {
        self.start
    }
}

/// A plan does not move money. Only a separately confirmed expense does.
/// Terms remain immutable; stop and replace a plan to change future amounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurringExpense {
    id: RecurringId,
    name: AccountName,
    amount: PositiveMoney,
    category: Category,
    account: Option<AccountId>,
    due: MonthlyDue,
    stopped_from: Option<Month>,
}
impl RecurringExpense {
    pub fn new(
        id: RecurringId,
        name: AccountName,
        amount: PositiveMoney,
        category: Category,
        account: Option<AccountId>,
        due: MonthlyDue,
    ) -> Result<Self, DomainError> {
        if !Category::EXPENSE.contains(&category) {
            return Err(DomainError::InvalidCategory);
        }
        Ok(Self {
            id,
            name,
            amount,
            category,
            account,
            due,
            stopped_from: None,
        })
    }
    pub fn stop_from(mut self, month: Month) -> Result<Self, DomainError> {
        if month < self.due.start {
            return Err(DomainError::InvalidRecurring);
        }
        self.stopped_from = Some(month);
        Ok(self)
    }
    pub fn with_installments(mut self, count: Option<u32>) -> Result<Self, DomainError> {
        self.due = self.due.with_installments(count)?;
        Ok(self)
    }
    pub fn id(&self) -> RecurringId {
        self.id
    }
    pub fn name(&self) -> &AccountName {
        &self.name
    }
    pub fn amount(&self) -> PositiveMoney {
        self.amount
    }
    pub fn category(&self) -> Category {
        self.category
    }
    pub fn account(&self) -> Option<AccountId> {
        self.account
    }
    pub fn without_account(mut self) -> Self {
        self.account = None;
        self
    }
    pub fn due(&self) -> MonthlyDue {
        self.due
    }
    pub fn stopped_from(&self) -> Option<Month> {
        self.stopped_from
    }
    pub fn occurs_in(&self, month: Month) -> bool {
        self.due.number_in(month).is_some() && self.stopped_from.is_none_or(|end| month < end)
    }
}
