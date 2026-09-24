//! Editable receipt text has its own grammar; OCR never becomes an AI instruction.
use crate::*;
use ledger_domain::*;

pub const MAX_RECEIPT_IMAGES: usize = 8;
pub const MAX_RECEIPT_BATCH_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_RECEIPT_PROMPT_CHARS: usize = 160_000;
const START: &str = "[ใบเสร็จ ";
const END: &str = "[จบใบเสร็จ]";

pub fn is_receipt_prompt(text: &str) -> bool {
    text.contains(START) || text.contains(END)
}

/// JSON quoting keeps OCR descriptions on one line, including literal delimiters.
pub fn format_receipt_prompt(
    analysis: &ReceiptAnalysis,
    today: EntryDate,
    number: usize,
) -> String {
    let input = analysis.draft(today);
    let mut text = format!(
        "[ใบเสร็จ {number}]\nยอดสุทธิ: {}\nวันที่: {}\nบัญชี: \nหมวด: \nหมายเหตุ: \n",
        input.amount, input.date
    );
    if let Some(receipt) = input.receipt {
        for line in receipt.lines {
            let description = serde_json::Value::String(line.description).to_string();
            let kind = line.kind.map(ReceiptLineKind::label).unwrap_or("ตรวจวิธีคิด");
            text.push_str(&format!("{kind}: {description} | {}\n", line.amount));
        }
    }
    text.push_str(END);
    text
}

fn invalid() -> AppError {
    AppError::Input("รูปแบบใบเสร็จไม่ครบ ตรวจหัว [ใบเสร็จ เลขที่] ท้าย [จบใบเสร็จ] และชื่อช่องเดิมให้ครบ แต่ละบรรทัดสินค้าใช้ วิธีคิด: \"ชื่อสินค้า\" | ยอดบาท".into())
}

pub fn resolve_receipt_prompt(
    text: &str,
    today: EntryDate,
    accounts: &[Account],
) -> Result<QuickResolution, AppError> {
    if text.chars().count() > MAX_RECEIPT_PROMPT_CHARS {
        return Err(AppError::Input(
            "ข้อความใบเสร็จยาวเกินไป แบ่งอัปโหลดเป็นชุดเล็กลง".into(),
        ));
    }
    let mut drafts = Vec::new();
    let mut ordinary = String::new();
    let mut lines = text.lines().peekable();
    let mut numbers = std::collections::HashSet::new();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if !line.starts_with(START) {
            if line == END {
                return Err(invalid());
            }
            ordinary.push_str(line);
            ordinary.push('\n');
            continue;
        }
        append_ordinary(&mut drafts, &mut ordinary, today, accounts)?;
        let number: usize = line
            .strip_prefix(START)
            .and_then(|s| s.strip_suffix(']'))
            .and_then(|s| s.parse().ok())
            .filter(|n| *n > 0 && *n <= MAX_RECEIPT_IMAGES)
            .ok_or_else(invalid)?;
        if !numbers.insert(number) {
            return Err(invalid());
        }
        let mut input = EntryInput::empty(today);
        let mut receipt = ReceiptInput::default();
        let mut fields = std::collections::HashSet::new();
        let mut closed = false;
        let mut guidance = vec![format!(
            "ใบเสร็จ {number}: ตรวจยอด วันที่ และทุกรายการกับภาพก่อนยืนยัน"
        )];
        for line in lines.by_ref() {
            let line = line.trim();
            if line == END {
                closed = true;
                break;
            }
            if line.is_empty() {
                continue;
            }
            let (key, value) = line.split_once(':').ok_or_else(invalid)?;
            let value = value.trim();
            if ["ยอดสุทธิ", "วันที่", "บัญชี", "หมวด", "หมายเหตุ"].contains(&key)
            {
                if !fields.insert(key.to_owned()) {
                    return Err(invalid());
                }
                match key {
                    "ยอดสุทธิ" => input.amount = value.into(),
                    "วันที่" => input.date = value.into(),
                    "บัญชี" => {
                        input.account = accounts
                            .iter()
                            .find(|a| !a.is_archived() && a.name().key() == value.to_lowercase())
                            .map(Account::id);
                        if !value.is_empty() && input.account.is_none() {
                            guidance.push(format!("ไม่พบบัญชี “{value}” เลือกหรือเพิ่มบัญชีนี้ก่อนบันทึก"));
                        }
                    }
                    "หมวด" => {
                        input.category = Category::EXPENSE
                            .iter()
                            .copied()
                            .find(|c| c.label() == value || c.code() == value)
                    }
                    "หมายเหตุ" => input.note = value.into(),
                    _ => {}
                }
            } else {
                let kind = ReceiptLineKind::ALL.into_iter().find(|k| k.label() == key);
                if kind.is_none() && key != "ตรวจวิธีคิด" {
                    return Err(invalid());
                }
                let (description, amount) = value.rsplit_once('|').ok_or_else(invalid)?;
                let description: String =
                    serde_json::from_str(description.trim()).map_err(|_| invalid())?;
                if description.chars().count() > 120 || receipt.lines.len() >= MAX_RECEIPT_LINES {
                    return Err(invalid());
                }
                receipt.lines.push(ReceiptLineInput {
                    description,
                    amount: amount.trim().into(),
                    kind,
                });
            }
        }
        if !closed || fields.len() != 5 {
            return Err(invalid());
        }
        input.receipt = Some(receipt);
        drafts.push(ModelDraft {
            input,
            guidance: guidance.join(" · "),
        });
        check_count(&drafts)?;
    }
    append_ordinary(&mut drafts, &mut ordinary, today, accounts)?;
    check_count(&drafts)?;
    Ok(QuickResolution::Batch {
        source: text.into(),
        drafts,
    })
}
fn append_ordinary(
    drafts: &mut Vec<ModelDraft>,
    text: &mut String,
    today: EntryDate,
    accounts: &[Account],
) -> Result<(), AppError> {
    if text.trim().is_empty() {
        text.clear();
        return Ok(());
    }
    match resolve_entry_text(text.trim(), today, accounts)? {
        QuickResolution::Draft { input, guidance } => drafts.push(ModelDraft { input, guidance }),
        QuickResolution::Batch {
            drafts: entries, ..
        } => drafts.extend(entries),
        _ => {
            return Err(AppError::Input(
                "แยกคำสั่งสรุปหรือจัดการบัญชีออกจากชุดใบเสร็จก่อน ทุกใบยังไม่ได้บันทึก".into(),
            ));
        }
    }
    text.clear();
    check_count(drafts)
}
fn check_count(drafts: &[ModelDraft]) -> Result<(), AppError> {
    if drafts.is_empty() || drafts.len() > MAX_BATCH_ENTRIES {
        Err(AppError::Input(
            "ตรวจได้ครั้งละไม่เกิน 8 รายการ รวมข้อความและใบเสร็จ แบ่งบันทึกเป็นชุดเล็กลง".into(),
        ))
    } else {
        Ok(())
    }
}

/// Remove complete attachment blocks without deleting ordinary typed transactions.
pub fn without_receipt_blocks(text: &str) -> String {
    let mut output = String::new();
    let mut pending = String::new();
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if pending.is_empty() && trimmed.starts_with(START) && trimmed.ends_with(']') {
            pending.push_str(line);
        } else if !pending.is_empty() {
            pending.push_str(line);
            if trimmed == END {
                pending.clear();
            }
        } else {
            output.push_str(line);
        }
    }
    output.push_str(&pending); // An unfinished edited block stays available to fix.
    output.trim().to_owned()
}
