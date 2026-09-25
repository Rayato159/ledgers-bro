use crate::{DomainError, EntryDate, Month};

/// Immutable monthly statement terms. Entries on the closing date belong to
/// that statement; a missing calendar day is clamped to the month's last day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreditCardCycle {
    closing_day: u32,
    payment_day: u32,
}

impl CreditCardCycle {
    pub fn new(closing_day: u32, payment_day: u32) -> Result<Self, DomainError> {
        if !(1..=31).contains(&closing_day) || !(1..=31).contains(&payment_day) {
            return Err(DomainError::InvalidCreditCycle);
        }
        Ok(Self {
            closing_day,
            payment_day,
        })
    }

    pub const fn closing_day(self) -> u32 {
        self.closing_day
    }
    pub const fn payment_day(self) -> u32 {
        self.payment_day
    }

    pub fn statement_for(self, date: EntryDate) -> Result<CreditStatementDates, DomainError> {
        let mut month = Month::of(date)?;
        if date > month.on_day(self.closing_day)? {
            month = month.shifted(1)?;
        }
        let closing = month.on_day(self.closing_day)?;
        let mut due_month = month;
        // Clamping can make two different configured days coincide in February.
        if due_month.on_day(self.payment_day)? <= closing {
            due_month = due_month.shifted(1)?;
        }
        Ok(CreditStatementDates {
            closing,
            due: due_month.on_day(self.payment_day)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CreditStatementDates {
    pub closing: EntryDate,
    pub due: EntryDate,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    #[test]
    fn closing_is_inclusive_and_year_rollover_uses_calendar_months() {
        let cycle = CreditCardCycle::new(20, 5).expect("cycle");
        for (day, closing, due) in [
            ("2026-09-19", "2026-09-20", "2026-10-05"),
            ("2026-09-20", "2026-09-20", "2026-10-05"),
            ("2026-09-21", "2026-10-20", "2026-11-05"),
            ("2026-12-21", "2027-01-20", "2027-02-05"),
        ] {
            let dates = cycle
                .statement_for(day.parse().expect("date"))
                .expect("dates");
            assert_eq!(dates.closing.to_string(), closing);
            assert_eq!(dates.due.to_string(), due);
        }
    }
    #[test]
    fn all_valid_day_pairs_handle_short_and_leap_months_without_due_before_closing() {
        for closing in 1..=31 {
            for payment in 1..=31 {
                let cycle = CreditCardCycle::new(closing, payment).expect("cycle");
                for day in [
                    "2024-02-28",
                    "2024-02-29",
                    "2026-02-28",
                    "2026-04-30",
                    "2026-12-31",
                ] {
                    let day = day.parse().expect("date");
                    let dates = cycle.statement_for(day).expect("dates");
                    assert!(dates.closing >= day);
                    assert!(dates.due > dates.closing);
                }
            }
        }
        let dates = CreditCardCycle::new(31, 31)
            .expect("cycle")
            .statement_for("2024-02-29".parse().expect("date"))
            .expect("dates");
        assert_eq!(dates.closing.to_string(), "2024-02-29");
        assert_eq!(dates.due.to_string(), "2024-03-31");
        assert!(CreditCardCycle::new(0, 5).is_err());
        assert!(CreditCardCycle::new(20, 32).is_err());
    }
}
