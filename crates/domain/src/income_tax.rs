use crate::{DomainError, Money, PositiveMoney};

/// Section 40 classification explicitly chosen from the taxpayer's evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeSection(u8);
impl IncomeSection {
    pub fn new(value: u8) -> Result<Self, DomainError> {
        if (1..=8).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidIncomeTax)
        }
    }
    pub const fn number(self) -> u8 {
        self.0
    }
    pub const fn index(self) -> usize {
        (self.0 - 1) as usize
    }
}

/// Tax evidence attached to a cash receipt. It does not turn VAT into PIT
/// income or treat withholding as money actually deposited into the account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncomeTax {
    section: IncomeSection,
    gross: PositiveMoney,
    withholding: Money,
    vat: Money,
    other_deductions: Money,
}
impl IncomeTax {
    pub fn new(
        section: IncomeSection,
        gross: PositiveMoney,
        withholding: Money,
        vat: Money,
        other_deductions: Money,
    ) -> Result<Self, DomainError> {
        if [withholding, vat, other_deductions]
            .iter()
            .any(|m| *m < Money::ZERO)
            || withholding.checked_add(other_deductions)? > gross.money()
        {
            return Err(DomainError::InvalidIncomeTax);
        }
        Ok(Self {
            section,
            gross,
            withholding,
            vat,
            other_deductions,
        })
    }
    pub fn net_received(self) -> Result<Money, DomainError> {
        self.gross
            .money()
            .checked_add(self.vat)?
            .checked_sub(self.withholding)?
            .checked_sub(self.other_deductions)
    }
    pub const fn section(self) -> IncomeSection {
        self.section
    }
    pub const fn gross(self) -> Money {
        self.gross.money()
    }
    pub const fn withholding(self) -> Money {
        self.withholding
    }
    pub const fn vat(self) -> Money {
        self.vat
    }
    pub const fn other_deductions(self) -> Money {
        self.other_deductions
    }
}
