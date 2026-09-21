use crate::DomainError;
use chrono::{Datelike, NaiveDate};
use std::{fmt, str::FromStr};
use uuid::Uuid;

macro_rules! identity {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Uuid);
        impl $name {
            pub fn from_uuid(value: Uuid) -> Result<Self, DomainError> {
                if value.is_nil() {
                    return Err(DomainError::InvalidId);
                }
                Ok(Self(value))
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl FromStr for $name {
            type Err = DomainError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::from_uuid(Uuid::parse_str(value).map_err(|_| DomainError::InvalidId)?)
            }
        }
    };
}
identity!(AccountId);
identity!(EntryId);
identity!(SubmissionId);

/// THB in satang. The bound applies to individual and aggregated amounts.
/// No float conversion is provided; presentation must preserve exact cents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(i64);

impl Money {
    pub const ZERO: Self = Self(0);
    pub const MAX_MINOR: i64 = 9_000_000_000_000;
    pub fn from_minor(value: i64) -> Result<Self, DomainError> {
        if !(-Self::MAX_MINOR..=Self::MAX_MINOR).contains(&value) {
            return Err(DomainError::MoneyOverflow);
        }
        Ok(Self(value))
    }
    pub const fn minor(self) -> i64 {
        self.0
    }
    pub fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        Self::from_minor(
            self.0
                .checked_add(other.0)
                .ok_or(DomainError::MoneyOverflow)?,
        )
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        Self::from_minor(
            self.0
                .checked_sub(other.0)
                .ok_or(DomainError::MoneyOverflow)?,
        )
    }
    pub const fn negated(self) -> Self {
        Self(-self.0)
    }
}

impl FromStr for Money {
    type Err = DomainError;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let text = text.trim();
        if text.is_empty() || text.len() > 18 {
            return Err(DomainError::InvalidMoney);
        }
        let (negative, digits) = text.strip_prefix('-').map_or((false, text), |v| (true, v));
        let mut parts = digits.split('.');
        let whole = parts.next().ok_or(DomainError::InvalidMoney)?;
        let fraction = parts.next();
        if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(DomainError::InvalidMoney);
        }
        let cents = match fraction {
            None => 0,
            Some(v) if (1..=2).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_digit()) => {
                let n: i64 = v.parse().map_err(|_| DomainError::InvalidMoney)?;
                if v.len() == 1 { n * 10 } else { n }
            }
            _ => return Err(DomainError::InvalidMoney),
        };
        let whole: i64 = whole.parse().map_err(|_| DomainError::MoneyOverflow)?;
        let value = whole
            .checked_mul(100)
            .and_then(|v| v.checked_add(cents))
            .ok_or(DomainError::MoneyOverflow)?;
        Self::from_minor(if negative { -value } else { value })
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unsigned = self.0.unsigned_abs();
        write!(
            f,
            "{}{}.{:02}",
            if self.0 < 0 { "-" } else { "" },
            unsigned / 100,
            unsigned % 100
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositiveMoney(Money);
impl PositiveMoney {
    pub fn new(value: Money) -> Result<Self, DomainError> {
        if value.minor() <= 0 {
            return Err(DomainError::NonPositiveAmount);
        }
        Ok(Self(value))
    }
    pub const fn money(self) -> Money {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountName(String);
impl AccountName {
    pub fn new(value: &str) -> Result<Self, DomainError> {
        if value.chars().any(char::is_control) {
            return Err(DomainError::InvalidName);
        }
        let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
        if value.is_empty() || value.chars().count() > 60 {
            return Err(DomainError::InvalidName);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn key(&self) -> String {
        self.0.to_lowercase()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Note(String);
impl Note {
    pub const MAX_CHARS: usize = 20_000;
    pub fn new(value: &str) -> Result<Self, DomainError> {
        if value.chars().count() > Self::MAX_CHARS
            || value
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t')
        {
            return Err(DomainError::InvalidNote);
        }
        Ok(Self(value.trim().to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntryDate(NaiveDate);
impl EntryDate {
    pub fn new(value: NaiveDate) -> Result<Self, DomainError> {
        if !(1900..=9999).contains(&value.year()) {
            return Err(DomainError::InvalidDate);
        }
        Ok(Self(value))
    }
    pub fn previous_day(self) -> Result<Self, DomainError> {
        Self::new(self.0.pred_opt().ok_or(DomainError::InvalidDate)?)
    }
    pub const fn date(self) -> NaiveDate {
        self.0
    }
    pub fn month_key(self) -> (i32, u32) {
        (self.0.year(), self.0.month())
    }
}
impl fmt::Display for EntryDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl FromStr for EntryDate {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 10 {
            return Err(DomainError::InvalidDate);
        }
        Self::new(
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| DomainError::InvalidDate)?,
        )
    }
}
