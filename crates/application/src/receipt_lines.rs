//! Conservative OCR candidates. Line amounts and tax treatment require review.
use crate::{AppError, ReceiptLineInput, receipt::parse_amount};
use ledger_domain::{MAX_RECEIPT_LINES, Money, ReceiptLineKind};

pub(crate) fn extract_lines(
    text: &str,
    foreign_currency: bool,
) -> Result<Vec<ReceiptLineInput>, AppError> {
    let mut lines = Vec::new();
    let mut after_total = false;
    for raw in text.lines() {
        let line = raw.trim();
        let lower = line.to_lowercase();
        let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
        if [
            "taxid",
            "taxinvoice",
            "vatno",
            "vatnumber",
            "ใบกำกับภาษี",
            "เลขผู้เสียภาษี",
            "totalqty",
        ]
        .iter()
        .any(|prefix| compact.starts_with(prefix))
        {
            continue;
        }
        if [
            "grandtotal",
            "nettotal",
            "ยอดสุทธิ",
            "ยอดรวม",
            "ยอดชำระ",
            "รวมทั้งสิ้น",
            "รวมเงิน",
            "total",
        ]
        .iter()
        .any(|prefix| compact.starts_with(prefix))
            && !compact.starts_with("totalvat")
        {
            after_total = true;
            continue;
        }
        let discount = ["discount", "ส่วนลด", "ลดราคา"]
            .iter()
            .any(|prefix| compact.starts_with(prefix));
        let tax = ["vat", "tax", "ภาษี"]
            .iter()
            .any(|prefix| compact.starts_with(prefix));
        let service = ["service", "ค่าบริการ", "ค่าจัดส่ง", "shipping"]
            .iter()
            .any(|prefix| compact.starts_with(prefix));
        let rounding = ["round", "ปัดเศษ"]
            .iter()
            .any(|prefix| compact.starts_with(prefix));
        let adjustment = discount || tax || service || rounding;
        if !adjustment
            && (after_total
                || [
                    "subtotal",
                    "sub-total",
                    "รวมก่อน",
                    "รวมสินค้า",
                    "ยอดก่อน",
                    "ก่อนภาษี",
                    "cash",
                    "change",
                    "เงินสด",
                    "เงินทอน",
                    "รับเงิน",
                    "เงินรับ",
                    "paid",
                    "payment",
                    "บัตร",
                    "credit",
                    "visa",
                    "mastercard",
                    "promptpay",
                    "พร้อมเพย์",
                    "receipt",
                    "invoice",
                    "ใบเสร็จ",
                    "เลขที่",
                    "วันที่",
                    "date",
                    "time",
                    "เวลา",
                    "tel",
                    "โทร",
                    "taxid",
                    "เลขประจำตัว",
                    "ที่อยู่",
                    "address",
                    "สาขา",
                    "branch",
                    "table",
                    "โต๊ะ",
                    "cashier",
                    "พนักงาน",
                    "สมาชิก",
                    "member",
                    "points",
                    "แต้ม",
                    "จำนวนรวม",
                    "รวมจำนวน",
                    "จำนวนรายการ",
                    "totalqty",
                    "รวมภาษี",
                    "thank",
                    "ขอบคุณ",
                ]
                .iter()
                .any(|prefix| compact.starts_with(prefix)))
        {
            continue;
        }
        let mut trimmed = line.trim_end_matches(['฿', ' ']);
        for suffix in ["บาท", "THB", "thb"] {
            if let Some(value) = trimmed.strip_suffix(suffix) {
                trimmed = value.trim_end();
            }
        }
        let Some(split) = trimmed.rfind(char::is_whitespace) else {
            continue;
        };
        let token = trimmed[split..].trim().trim_start_matches('฿');
        let absolute = token.strip_prefix('-').unwrap_or(token);
        let Some(mut amount) = parse_amount(absolute)
            .or_else(|| matches!(absolute, "0" | "0.0" | "0.00").then_some(Money::ZERO))
        else {
            continue;
        };
        let description = trimmed[..split].trim().trim_end_matches(['=', ':']).trim();
        if !description.chars().any(|c| c.is_alphabetic()) {
            continue;
        }
        let included = compact.contains("included")
            || compact.contains("รวมในราคา")
            || compact.contains("รวมแล้ว");
        let kind = if discount {
            Some(ReceiptLineKind::Discount)
        } else if rounding {
            Some(ReceiptLineKind::Rounding)
        } else if (tax || service) && included {
            Some(ReceiptLineKind::IncludedCharge)
        } else if tax || service {
            None
        } else {
            Some(ReceiptLineKind::Item)
        };
        if token.starts_with('-') && !discount {
            if rounding {
                amount = amount.negated();
            } else {
                continue;
            }
        }
        lines.push(ReceiptLineInput {
            description: description.to_owned(),
            // Never treat a foreign-currency line as its THB equivalent.
            amount: if foreign_currency {
                String::new()
            } else {
                amount.to_string()
            },
            kind,
        });
        if lines.len() > MAX_RECEIPT_LINES {
            return Err(AppError::Input(
                "ใบเสร็จรองรับไม่เกิน 100 บรรทัด กรุณาแยกรายการ".into(),
            ));
        }
    }
    Ok(lines)
}
