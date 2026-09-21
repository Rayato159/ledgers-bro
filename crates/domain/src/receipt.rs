use crate::{DomainError, Money, Note, PositiveMoney};

pub const MAX_RECEIPT_LINES: usize = 100;
pub const MAX_RECEIPT_DESCRIPTION: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptLineKind {
    Item,
    Discount,
    AddedTax,
    ServiceCharge,
    IncludedCharge,
    Rounding,
}
impl ReceiptLineKind {
    pub const ALL: [Self; 6] = [
        Self::Item,
        Self::Discount,
        Self::AddedTax,
        Self::ServiceCharge,
        Self::IncludedCharge,
        Self::Rounding,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Item => "สินค้า / บริการ",
            Self::Discount => "ส่วนลด",
            Self::AddedTax => "ภาษีที่บวกเพิ่ม",
            Self::ServiceCharge => "ค่าบริการที่บวกเพิ่ม",
            Self::IncludedCharge => "ภาษี/ค่าบริการที่รวมในราคาแล้ว",
            Self::Rounding => "ปัดเศษ (+ / −)",
        }
    }
    pub const fn code(self) -> &'static str {
        match self {
            Self::Item => "item",
            Self::Discount => "discount",
            Self::AddedTax => "added-tax",
            Self::ServiceCharge => "service",
            Self::IncludedCharge => "included-tax",
            Self::Rounding => "rounding",
        }
    }
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
    }
}

/// A value object: the line's meaning is its description, amount and treatment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLine {
    description: String,
    amount: Money,
    kind: ReceiptLineKind,
}
impl ReceiptLine {
    pub fn new(
        description: &str,
        amount: Money,
        kind: ReceiptLineKind,
    ) -> Result<Self, DomainError> {
        if description.trim().is_empty()
            || description.chars().count() > MAX_RECEIPT_DESCRIPTION
            || description.chars().any(char::is_control)
            || (amount < Money::ZERO && kind != ReceiptLineKind::Rounding)
        {
            return Err(DomainError::InvalidReceiptLine);
        }
        Ok(Self {
            description: description.trim().to_owned(),
            amount,
            kind,
        })
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub const fn amount(&self) -> Money {
        self.amount
    }
    pub const fn kind(&self) -> ReceiptLineKind {
        self.kind
    }
    pub fn contribution(&self) -> Money {
        match self.kind {
            ReceiptLineKind::Discount => self.amount.negated(),
            ReceiptLineKind::IncludedCharge => Money::ZERO,
            _ => self.amount,
        }
    }
}

/// Only constructible when all line contributions match the payment exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptBreakdown {
    lines: Vec<ReceiptLine>,
    total: PositiveMoney,
}
impl ReceiptBreakdown {
    pub fn new(lines: Vec<ReceiptLine>, total: PositiveMoney) -> Result<Self, DomainError> {
        if lines.is_empty()
            || lines.len() > MAX_RECEIPT_LINES
            || !lines.iter().any(|line| line.kind == ReceiptLineKind::Item)
        {
            return Err(DomainError::InvalidReceiptLines);
        }
        let mut calculated = Money::ZERO;
        let mut included_charges = Money::ZERO;
        for line in &lines {
            calculated = calculated.checked_add(line.contribution())?;
            if line.kind == ReceiptLineKind::IncludedCharge {
                included_charges = included_charges.checked_add(line.amount)?;
            }
        }
        if calculated != total.money() {
            return Err(DomainError::ReceiptTotalMismatch {
                calculated,
                total: total.money(),
            });
        }
        if included_charges > total.money() {
            return Err(DomainError::InvalidIncludedCharge);
        }
        Ok(Self { lines, total })
    }
    pub fn lines(&self) -> &[ReceiptLine] {
        &self.lines
    }
    pub const fn total(&self) -> Money {
        self.total.money()
    }

    /// The persisted note is generated from the reconciled values, never OCR prose.
    pub fn note(&self, extra: &str) -> Result<Note, DomainError> {
        let mut text = String::from("รายการจากใบเสร็จ\n");
        for line in &self.lines {
            let amount = if line.kind == ReceiptLineKind::IncludedCharge {
                format!("{} บาท (รวมในราคาแล้ว ไม่บวกซ้ำ)", line.amount)
            } else {
                format!("{} บาท", line.contribution())
            };
            text.push_str(&format!("• {} — {}\n", line.description, amount));
        }
        text.push_str(&format!("ยอดสุทธิ {} บาท", self.total()));
        if !extra.trim().is_empty() {
            text.push_str("\n\nหมายเหตุ: ");
            text.push_str(extra.trim());
        }
        Note::new(&text)
    }
}
