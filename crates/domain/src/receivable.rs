use crate::*;

/// Principal owed by another person. Repayments are separate immutable journals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receivable {
    id: ReceivableId,
    debtor: AccountName,
    description: Note,
    total: PositiveMoney,
    opened: EntryDate,
    start: Month,
    day: Option<u32>,
    installments: Option<u32>,
}
impl Receivable {
    pub fn new(
        id: ReceivableId,
        debtor: AccountName,
        description: Note,
        total: PositiveMoney,
        opened: EntryDate,
        terms: CollectionTerms,
    ) -> Result<Self, DomainError> {
        if description.as_str().trim().is_empty()
            || description.as_str().chars().count() > 300
            || terms.start < Month::of(opened)?
            || terms
                .installments
                .is_some_and(|n| i64::from(n) > total.money().minor())
        {
            return Err(DomainError::InvalidReceivable);
        }
        if let Some(day) = terms.day
            && terms.start.on_day(day)? < opened
        {
            return Err(DomainError::InvalidReceivable);
        }
        Ok(Self {
            id,
            debtor,
            description,
            total,
            opened,
            start: terms.start,
            day: terms.day,
            installments: terms.installments,
        })
    }
    pub fn id(&self) -> ReceivableId {
        self.id
    }
    pub fn debtor(&self) -> &AccountName {
        &self.debtor
    }
    pub fn description(&self) -> &Note {
        &self.description
    }
    pub fn total(&self) -> PositiveMoney {
        self.total
    }
    pub fn opened(&self) -> EntryDate {
        self.opened
    }
    pub fn start(&self) -> Month {
        self.start
    }
    pub fn day(&self) -> Option<u32> {
        self.day
    }
    pub fn installments(&self) -> Option<u32> {
        self.installments
    }
    /// Equal principal installments, with residual satang in the final one.
    pub fn installment_amount(&self, number: u32) -> Result<Money, DomainError> {
        let count = self.installments.ok_or(DomainError::InvalidInstallments)?;
        if !(1..=count).contains(&number) {
            return Err(DomainError::InvalidInstallments);
        }
        let base = self.total.money().minor() / i64::from(count);
        Money::from_minor(if number == count {
            self.total.money().minor() - base * i64::from(count - 1)
        } else {
            base
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectionTerms {
    start: Month,
    day: Option<u32>,
    installments: Option<u32>,
}
impl CollectionTerms {
    pub fn new(
        start: Month,
        day: Option<u32>,
        installments: Option<u32>,
    ) -> Result<Self, DomainError> {
        MonthlyDue::new(day.unwrap_or(1), start)?.with_installments(installments)?;
        Ok(Self {
            start,
            day,
            installments,
        })
    }
}
