use crate::{AppError, EntryInput, TransactionKind};
use ledger_domain::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickResolution {
    Actions {
        source: String,
        drafts: Vec<crate::PromptDraft>,
    },
    Batch {
        source: String,
        drafts: Vec<ModelDraft>,
    },
    Draft {
        input: EntryInput,
        guidance: String,
    },
    Choices {
        source: String,
        drafts: Vec<ModelDraft>,
    },
    Summary,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDraft {
    pub input: EntryInput,
    pub guidance: String,
}

/// A small grammar, not a probabilistic model. The whole message must be consumed.
pub fn resolve_quick_entry(
    text: &str,
    today: EntryDate,
    accounts: &[Account],
) -> Result<QuickResolution, AppError> {
    if text.chars().count() > 1000 {
        return Err(AppError::Input("ข้อความยาวเกิน 1,000 ตัวอักษร".into()));
    }
    let tokens = tokenize(text)?;
    if tokens.as_slice() == ["ช่วยเหลือ"] {
        return Ok(QuickResolution::Help);
    }
    if tokens.as_slice() == ["สรุป", "เดือนนี้"] {
        return Ok(QuickResolution::Summary);
    }
    let mut input = EntryInput::empty(today);
    // Explicitly bounded shorthand. Unknown prose never becomes a guessed transaction.
    const SHORT_ITEMS: &[&str] = &["กาแฟ", "ข้าว", "อาหาร", "ขนม", "ชา", "น้ำ", "แท็กซี่", "ค่าเช่า"];
    // Thai ASR may omit word spaces or spell the baht unit. Consume the whole
    // known-item + numeric-amount phrase; never guess number words or ignore a tail.
    for item in SHORT_ITEMS {
        if let Some(rest) = text.trim().strip_prefix(item) {
            let rest = rest.trim();
            let amount = rest.strip_suffix("บาท").unwrap_or(rest).trim();
            input.amount = PositiveMoney::new(amount.parse()?)?.money().to_string();
            input.note = (*item).into();
            return Ok(QuickResolution::Draft {
                input,
                guidance: "เลือกบัญชีและหมวดก่อนตรวจรายการ ยังไม่ได้บันทึก".into(),
            });
        }
    }
    if tokens.len() == 2 && SHORT_ITEMS.contains(&tokens[0].as_str()) {
        input.amount = PositiveMoney::new(tokens[1].parse()?)?.money().to_string();
        input.note = tokens[0].clone();
        return Ok(QuickResolution::Draft {
            input,
            guidance: "เลือกบัญชีและหมวดก่อนตรวจรายการ ยังไม่ได้บันทึก".into(),
        });
    }
    if tokens.len() < 6 {
        return Err(grammar_error());
    }
    input.kind = match tokens[0].as_str() {
        "จ่าย" => TransactionKind::Expense,
        "รับ" => TransactionKind::Income,
        "โอน" => TransactionKind::Transfer,
        _ => return Err(grammar_error()),
    };
    input.amount = PositiveMoney::new(tokens[1].parse()?)?.money().to_string();
    let preposition = if input.kind == TransactionKind::Income {
        "เข้า"
    } else {
        "จาก"
    };
    let separator = if input.kind == TransactionKind::Transfer {
        "ไป"
    } else {
        "หมวด"
    };
    if tokens[2] != preposition || tokens[4] != separator {
        return Err(grammar_error());
    }
    let find_account = |name: &str| {
        accounts
            .iter()
            .find(|a| !a.is_archived() && a.name().key() == name.to_lowercase())
            .map(Account::id)
    };
    input.account = find_account(&tokens[3]);
    let mut guidance = Vec::new();
    if input.account.is_none() {
        guidance.push(format!("ไม่พบบัญชี “{}” ตรงชื่อ กรุณาเลือกบัญชี", tokens[3]));
    }
    if input.kind == TransactionKind::Transfer {
        input.destination = find_account(&tokens[5]);
        if input.destination.is_none() {
            guidance.push(format!("กรุณาเลือกบัญชีปลายทางแทน “{}”", tokens[5]));
        }
    } else {
        if tokens[5] == "หนี้" {
            return Err(DomainError::DebtNeedsTransfer.into());
        }
        let categories: &[Category] = if input.kind == TransactionKind::Income {
            &Category::INCOME
        } else {
            &Category::EXPENSE
        };
        input.category = categories.iter().copied().find(|c| c.label() == tokens[5]);
        if input.category.is_none() {
            guidance.push(format!("ไม่พบหมวด “{}” กรุณาเลือกจากตัวเลือก", tokens[5]));
        }
    }
    let mut cursor = 6;
    if tokens.get(cursor).is_some_and(|t| t == "วันที่") {
        let date = tokens.get(cursor + 1).ok_or_else(grammar_error)?;
        input.date = match date.as_str() {
            "วันนี้" => today,
            "เมื่อวาน" => today.previous_day()?,
            _ => date.parse()?,
        }
        .to_string();
        cursor += 2;
    }
    if tokens.get(cursor).is_some_and(|t| t == "โน้ต") {
        input.note = tokens.get(cursor + 1).ok_or_else(grammar_error)?.clone();
        cursor += 2;
    }
    if cursor != tokens.len() {
        return Err(grammar_error());
    }
    Note::new(&input.note)?;
    if input.kind == TransactionKind::Transfer
        && input.account.is_some()
        && input.account == input.destination
    {
        return Err(DomainError::SameAccountTransfer.into());
    }
    if guidance.is_empty() {
        guidance.push("ตรวจบัญชี ยอด และวันที่ก่อนยืนยัน ยังไม่ได้บันทึก".into());
    }
    Ok(QuickResolution::Draft {
        input,
        guidance: guidance.join(" · "),
    })
}

fn grammar_error() -> AppError {
    AppError::Input(
        "ยังอ่านคำสั่งนี้ไม่ได้ ลอง “จ่าย 80 จาก เงินสด หมวด อาหาร” หรือใช้แบบฟอร์ม ไม่ได้บันทึกข้อมูล".into(),
    )
}

fn tokenize(text: &str) -> Result<Vec<String>, AppError> {
    let mut result = Vec::new();
    let mut chars = text.trim().chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            continue;
        }
        let mut token = String::new();
        if ch == '"' {
            let mut closed = false;
            for ch in chars.by_ref() {
                if ch == '"' {
                    closed = true;
                    break;
                }
                token.push(ch);
            }
            if !closed || chars.peek().is_some_and(|c| !c.is_whitespace()) {
                return Err(grammar_error());
            }
        } else {
            token.push(ch);
            while chars.peek().is_some_and(|c| !c.is_whitespace()) {
                if let Some(ch) = chars.next() {
                    if ch == '"' {
                        return Err(grammar_error());
                    }
                    token.push(ch);
                }
            }
        }
        if token.is_empty() {
            return Err(grammar_error());
        }
        result.push(token);
    }
    Ok(result)
}
