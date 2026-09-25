use crate::{AppError, Dashboard, TaxWorksheet};
use chrono::Datelike;
use ledger_domain::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncomeTaxInput {
    pub section: Option<IncomeSection>,
    pub gross: String,
    pub withholding: String,
    pub vat: String,
    pub other_deductions: String,
}
impl Default for IncomeTaxInput {
    fn default() -> Self {
        Self {
            section: None,
            gross: String::new(),
            withholding: "0".into(),
            vat: "0".into(),
            other_deductions: "0".into(),
        }
    }
}
impl IncomeTaxInput {
    pub fn validate(&self) -> Result<IncomeTax, AppError> {
        Ok(IncomeTax::new(
            self.section.ok_or(DomainError::InvalidIncomeTax)?,
            PositiveMoney::new(self.gross.parse()?)?,
            self.withholding.parse()?,
            self.vat.parse()?,
            self.other_deductions.parse()?,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AnnualTaxIncome {
    pub incomes: [Money; 8],
    pub withholding: Money,
    pub count: usize,
}

/// Recompute from active receipt evidence; never store a second running total.
pub fn annual_tax_income(
    view: &Dashboard,
    buddhist_year: u16,
) -> Result<AnnualTaxIncome, AppError> {
    if view.currency != Currency::Thb {
        return Err(AppError::Input("ภาษีไทยใช้ได้กับสมุดสกุล THB เท่านั้น".into()));
    }
    let year = i32::from(buddhist_year) - 543;
    let mut result = AnnualTaxIncome::default();
    for entry in &view.entries {
        if entry.date().date().year() != year
            || entry.date() > view.today
            || view.reversed.contains(&entry.id())
        {
            continue;
        }
        if let Some(tax) = entry.income_tax() {
            result.incomes[tax.section().index()] =
                result.incomes[tax.section().index()].checked_add(tax.gross())?;
            result.withholding = result.withholding.checked_add(tax.withholding())?;
            result.count += 1;
        }
    }
    Ok(result)
}

/// Manual amounts are additional evidence only. Work on a copy so rendering or
/// recalculating never adds automatic totals back into the manual fields.
pub fn tax_worksheet_with_entries(
    manual: &TaxWorksheet,
    automatic: &AnnualTaxIncome,
) -> Result<TaxWorksheet, AppError> {
    let mut result = manual.clone();
    let add = |text: &str, amount: Money| -> Result<String, AppError> {
        let manual: Money = text.parse()?;
        if manual < Money::ZERO {
            return Err(DomainError::InvalidIncomeTax.into());
        }
        Ok(manual.checked_add(amount)?.to_string())
    };
    for (index, amount) in automatic.incomes.iter().enumerate() {
        result.incomes[index] = add(&manual.incomes[index], *amount)?;
    }
    result.withholding = add(&manual.withholding, automatic.withholding)?;
    Ok(result)
}
