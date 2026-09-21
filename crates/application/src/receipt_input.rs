use crate::AppError;
use ledger_domain::{Money, PositiveMoney, ReceiptBreakdown, ReceiptLine, ReceiptLineKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptLineInput {
    pub description: String,
    pub amount: String,
    /// None means the OCR line's financial treatment needs a user choice.
    pub kind: Option<ReceiptLineKind>,
}
impl Default for ReceiptLineInput {
    fn default() -> Self {
        Self {
            description: String::new(),
            amount: String::new(),
            kind: Some(ReceiptLineKind::Item),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReceiptInput {
    pub lines: Vec<ReceiptLineInput>,
    pub reviewed: bool,
}
impl ReceiptInput {
    pub fn reconcile(&self, total: &str) -> Result<ReceiptBreakdown, AppError> {
        let lines = self
            .lines
            .iter()
            .map(|line| {
                let kind = line.kind.ok_or_else(|| {
                    AppError::Input("เลือกวิธีคิดภาษี/ค่าบริการว่าเป็นยอดบวกเพิ่มหรือรวมในราคาแล้ว".into())
                })?;
                let amount: Money = line.amount.parse()?;
                Ok(ReceiptLine::new(&line.description, amount, kind)?)
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(ReceiptBreakdown::new(
            lines,
            PositiveMoney::new(total.parse()?)?,
        )?)
    }
    pub fn verified(&self, total: &str) -> Result<ReceiptBreakdown, AppError> {
        let breakdown = self.reconcile(total)?;
        if !self.reviewed {
            return Err(AppError::Input(
                "ตรวจทุกรายการกับภาพใบเสร็จ แล้วติ๊กยืนยันรายละเอียดก่อนบันทึก".into(),
            ));
        }
        Ok(breakdown)
    }
}
