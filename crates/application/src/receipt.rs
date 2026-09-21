//! OCR is untrusted input. This module proposes values; it never writes a ledger.
use crate::{AppError, EntryInput, ReceiptInput, ReceiptLineInput};
use ledger_domain::{EntryDate, Money};
use std::sync::{Arc, atomic::AtomicBool};

pub const MAX_RECEIPT_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_OCR_TEXT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptImage {
    bytes: Arc<[u8]>,
    mime: &'static str,
}
impl ReceiptImage {
    pub fn new(bytes: Vec<u8>) -> Result<Self, AppError> {
        if bytes.len() > MAX_RECEIPT_BYTES {
            return Err(AppError::Input("รูปใบเสร็จต้องไม่เกิน 32 MB".into()));
        }
        let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            "image/jpeg"
        } else if is_heif(&bytes) {
            "image/heif"
        } else {
            return Err(AppError::Input(
                "เลือกรูปใบเสร็จเป็น JPG, PNG หรือ HEIC/HEIF".into(),
            ));
        };
        Ok(Self {
            bytes: bytes.into(),
            mime,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn mime(&self) -> &'static str {
        self.mime
    }
}

// This is container identification, not proof that the image is decodable.
// Decode/limits/orientation belong to the platform image adapter.
fn is_heif(bytes: &[u8]) -> bool {
    let Some(header) = bytes.get(..16) else {
        return false;
    };
    let size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if &header[4..8] != b"ftyp" || !(16..=256).contains(&size) || !size.is_multiple_of(4) {
        return false;
    }
    let Some(container) = bytes.get(..size) else {
        return false;
    };
    let brands = std::iter::once(&container[8..12]).chain(container[16..].chunks_exact(4));
    let mut heif = false;
    for brand in brands {
        if matches!(brand, b"avif" | b"avis" | b"hevc" | b"hevx" | b"msf1") {
            return false; // AVIF and image sequences are outside the receipt contract.
        }
        heif |= matches!(brand, b"heic" | b"heix" | b"mif1");
    }
    heif
}

/// A platform adapter implements recognition. Cancellation must stop its worker.
pub trait ReceiptOcr: Send + Sync {
    fn recognize(&self, image: &ReceiptImage, cancel: &AtomicBool) -> Result<String, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptAmount {
    pub amount: Money,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptAnalysis {
    pub text: String,
    pub totals: Vec<ReceiptAmount>,
    pub dates: Vec<EntryDate>,
    pub foreign_currency: bool,
    pub lines: Vec<ReceiptLineInput>,
}
impl ReceiptAnalysis {
    pub fn draft(&self, today: EntryDate) -> EntryInput {
        let mut input = EntryInput::empty(today);
        // Today's date is a default, not a claim that OCR recognized it.
        input.date = if self.dates.len() == 1 {
            self.dates[0].to_string()
        } else {
            today.to_string()
        };
        if !self.foreign_currency && self.totals.len() == 1 {
            input.amount = self.totals[0].amount.to_string();
        }
        input.receipt = Some(ReceiptInput {
            lines: self.lines.clone(),
            reviewed: false,
        });
        input
    }
}

pub fn analyze_receipt(text: &str, today: EntryDate) -> Result<ReceiptAnalysis, AppError> {
    if text.len() > MAX_OCR_TEXT_BYTES {
        return Err(AppError::Input(
            "ข้อความในภาพยาวเกินไป ลองครอปเฉพาะใบเสร็จ".into(),
        ));
    }
    let normalized: String = text
        .chars()
        .map(|c| match c {
            '๐'..='๙' => char::from(b'0' + (c as u32 - '๐' as u32) as u8),
            _ => c,
        })
        .collect();
    let lower = normalized.to_lowercase();
    let foreign_currency = lower.contains(['$', '€', '£', '¥', '₩'])
        || lower.split(|c: char| !c.is_ascii_alphabetic()).any(|word| {
            [
                "usd", "eur", "gbp", "jpy", "cny", "sgd", "hkd", "aud", "cad", "krw",
            ]
            .contains(&word)
        });
    let mut totals = Vec::new();
    let mut dates = Vec::new();
    for line in normalized.lines() {
        let lower = line.to_lowercase();
        let compact: String = lower.chars().filter(|c| !c.is_whitespace()).collect();
        let excluded = [
            "subtotal",
            "sub-total",
            "รวมก่อน",
            "ก่อนภาษี",
            "ภาษี",
            "vat",
            "change",
            "เงินทอน",
            "รับเงิน",
            "cash",
            "เงินสด",
            "discount",
            "ส่วนลด",
            "จำนวน",
            "qty",
        ]
        .iter()
        .any(|s| compact.contains(s));
        let labelled_total = [
            "grandtotal",
            "nettotal",
            "total",
            "amountdue",
            "ยอดสุทธิ",
            "ยอดรวม",
            "รวมสุทธิ",
            "ยอดชำระ",
            "รวมทั้งสิ้น",
            "รวมเงิน",
        ]
        .iter()
        .any(|s| compact.contains(s));
        if labelled_total && !excluded {
            for token in line.split(|c: char| !c.is_ascii_digit() && !['.', ',', '-'].contains(&c))
            {
                if let Some(amount) = parse_amount(token)
                    && !totals
                        .iter()
                        .any(|existing: &ReceiptAmount| existing.amount == amount)
                    && totals.len() < 8
                {
                    totals.push(ReceiptAmount {
                        amount,
                        evidence: line.chars().take(160).collect(),
                    });
                }
            }
        }
        for token in line.split(|c: char| !c.is_ascii_digit() && !['/', '-'].contains(&c)) {
            if let Some(date) = parse_date(token).filter(|date| *date <= today)
                && !dates.contains(&date)
                && dates.len() < 8
            {
                dates.push(date);
            }
        }
    }
    Ok(ReceiptAnalysis {
        text: text.to_owned(),
        totals,
        dates,
        foreign_currency,
        lines: crate::receipt_lines::extract_lines(&normalized, foreign_currency)?,
    })
}

pub(crate) fn parse_amount(token: &str) -> Option<Money> {
    if token.is_empty() || token.starts_with('-') {
        return None;
    }
    // A comma is thousands grouping, never a guessed decimal separator.
    let integer = token.split('.').next()?;
    if token
        .split_once('.')
        .is_some_and(|(_, fraction)| fraction.contains(','))
    {
        return None;
    }
    if integer.contains(',') {
        let mut groups = integer.split(',');
        let first = groups.next()?;
        if first.is_empty() || first.len() > 3 || !groups.all(|part| part.len() == 3) {
            return None;
        }
    }
    let normalized = token.replace(',', "");
    let amount: Money = normalized.parse().ok()?;
    (amount.minor() > 0).then_some(amount)
}

fn parse_date(token: &str) -> Option<EntryDate> {
    let parts: Vec<_> = token.split(['/', '-']).collect();
    if parts.len() != 3 {
        return None;
    }
    let (year, month, day) = if parts[0].len() == 4 {
        (parts[0], parts[1], parts[2])
    } else if parts[2].len() == 4 {
        (parts[2], parts[1], parts[0])
    } else {
        return None;
    };
    let mut year: u32 = year.parse().ok()?;
    if year >= 2400 {
        year = year.checked_sub(543)?;
    }
    let month: u32 = month.parse().ok()?;
    let day: u32 = day.parse().ok()?;
    format!("{year:04}-{month:02}-{day:02}").parse().ok()
}
