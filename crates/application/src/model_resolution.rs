use crate::{
    AppError, EntryInput, ModelDraft, QuickResolution, TransactionKind,
    model_contract::{ProposalStatus, ProposedIntent, proposal_for_review},
};
use ledger_domain::{Account, Category, EntryDate, PositiveMoney};

/// Conservative v1 scope guard, not a full natural-language classifier.
/// Unrecognized wording still requires review; rejected input can use the form.
pub fn check_model_source(source: &str) -> Result<(), AppError> {
    if source.trim().is_empty() || source.chars().count() > 1000 || source.contains("<|") {
        return Err(AppError::Input(
            "พิมพ์รายการไม่เกิน 1,000 ตัวอักษร ไม่ใส่คำสั่งระบบ".into(),
        ));
    }
    let source = source.to_lowercase();
    if source.contains(['$', '€', '£', '¥', '₩'])
        || ["ดอลลาร์", "ยูโร", "หยวน", "เยน"]
            .iter()
            .any(|word| source.contains(word))
        || source
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|word| {
                [
                    "usd", "eur", "gbp", "jpy", "cny", "sgd", "hkd", "aud", "cad", "krw",
                ]
                .contains(&word)
            })
    {
        return Err(AppError::Input(
            "ข้อความมีสกุลเงินต่างประเทศ กรุณากรอกยอดบาทที่จ่ายจริงด้วยแบบฟอร์ม รุ่นนี้ AI ยังไม่แปลงสกุลเงิน".into(),
        ));
    }
    if [
        "ไม่ได้",
        "ยังไม่",
        "แค่ถาม",
        "สมมติ",
        "ถ้า",
        "ยกเลิก",
        "ลบรายการ",
        "แก้รายการ",
        "สองรายการ",
        "หลายรายการ",
        "2 รายการ",
        "did not",
        "didn't",
        "do not",
        "don't",
        "hypothetical",
    ]
    .iter()
    .any(|word| source.contains(word))
    {
        return Err(AppError::Input(
            "ข้อความมีคำปฏิเสธ สมมติ หรือหลายรายการ กรุณาระบุรายการที่เกิดขึ้นจริงทีละรายการ หรือใช้แบบฟอร์ม".into(),
        ));
    }
    Ok(())
}

/// Grounding is necessary, not proof of semantic correctness. Every model result
/// is an uncommitted choice, even when only one interpretation is returned.
pub fn resolve_model_proposal(
    source: &str,
    output: &str,
    today: EntryDate,
    accounts: &[Account],
) -> Result<QuickResolution, AppError> {
    check_model_source(source)?;
    let (proposal, discarded) = proposal_for_review(source, output)?;
    let unsupported = || {
        AppError::Input(
            "รายการนี้ยังตีความอัตโนมัติไม่ได้ กรุณาแยกเป็นรายการเดียวแล้วใช้แบบฟอร์ม ยังไม่ได้บันทึก".into(),
        )
    };
    if proposal.status == ProposalStatus::Unsupported {
        return Err(unsupported());
    }
    // Tax/base/net amounts are separate facts; never silently collapse them.
    let lower = source.to_lowercase();
    if ["vat", "withholding", "หัก ณ", "หักณ", "ภาษี", "ก่อนหัก", "หลังหัก"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Err(AppError::Input(
            "ข้อความมีรายละเอียดภาษี ต้องแยกฐาน VAT หัก ณ ที่จ่าย และยอดรับจ่ายสุทธิก่อน รุ่นนี้ยังไม่บันทึกชุดภาษีผ่าน AI"
                .into(),
        ));
    }
    let mut drafts = Vec::new();
    for candidate in proposal.candidates {
        // A model summary must not imply it computed an unsupported period.
        if candidate.intent == ProposedIntent::Summary {
            return Err(AppError::Input(
                "เปิดภาพรวมเพื่อดูรายรับรายจ่าย หรือใช้คำสั่ง “สรุป เดือนนี้”".into(),
            ));
        }
        let mut input = EntryInput::empty(today);
        input.kind = match candidate.intent {
            ProposedIntent::Expense => TransactionKind::Expense,
            ProposedIntent::Income => TransactionKind::Income,
            ProposedIntent::Transfer => TransactionKind::Transfer,
            ProposedIntent::Summary => return Err(unsupported()),
        };
        let mut guidance = vec!["AI เสนอรายการนี้ ตรวจเทียบข้อความเดิมก่อนเลือก ยังไม่ได้บันทึก".to_owned()];
        if discarded {
            guidance.push("AI เสนอชื่อหรือรายละเอียดที่ไม่ตรงข้อความเดิม จึงเว้นไว้ให้เลือกและกรอกเอง".into());
        }
        if let Some(currency) = candidate.currency_text.as_deref()
            && !["บาท", "฿", "thb"].contains(&currency.to_lowercase().as_str())
        {
            return Err(AppError::Input(
                "รุ่นนี้บันทึกยอดเป็นบาทเท่านั้น กรุณาระบุยอดบาทที่จ่ายจริง".into(),
            ));
        }
        if let Some(amount) = candidate.amount_text.as_deref() {
            // Matching a substring is insufficient: 80 must never be read out of
            // 800, -80, 80.50, or a thousands-grouped number.
            let numeric = |c: char| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+');
            let bounded = source.match_indices(amount).any(|(start, _)| {
                !source[..start].chars().next_back().is_some_and(numeric)
                    && !source[start + amount.len()..]
                        .chars()
                        .next()
                        .is_some_and(numeric)
            });
            if bounded && let Ok(value) = amount.parse().and_then(PositiveMoney::new) {
                input.amount = value.money().to_string();
            }
        }
        if input.amount.is_empty() {
            guidance.push("กรอกยอดเงินเป็นตัวเลข ระบบไม่แปลงหรือเดายอดที่อ่านไม่ชัด".into());
        }
        let find = |name: Option<&str>| {
            name.and_then(|name| {
                accounts
                    .iter()
                    .find(|a| !a.is_archived() && a.name().key() == name.to_lowercase())
                    .map(Account::id)
            })
        };
        input.account = find(candidate.account_text.as_deref());
        input.destination = find(candidate.destination_account_text.as_deref());
        if input.account.is_none() {
            guidance.push(match candidate.account_text {
                Some(name) => format!("ไม่พบบัญชีตรงกับ “{name}” เลือกบัญชีที่ถูกต้องจากรายการ"),
                None => "เลือกบัญชีที่ใช้กับรายการนี้".into(),
            });
        }
        if input.kind == TransactionKind::Transfer {
            if input.destination.is_none() {
                guidance.push("เลือกบัญชีปลายทาง".into());
            }
        } else {
            let categories: &[Category] = if input.kind == TransactionKind::Income {
                &Category::INCOME
            } else {
                &Category::EXPENSE
            };
            input.category = candidate
                .category_text
                .as_deref()
                .and_then(|text| categories.iter().find(|c| c.label() == text).copied());
            if input.category.is_none() {
                guidance.push("เลือกหมวดหมู่ให้ตรงกับรายการ".into());
            }
        }
        if let Some(date) = candidate.date_text {
            let date = match date.as_str() {
                "วันนี้" => Some(today),
                "เมื่อวาน" => today.previous_day().ok(),
                _ => date.parse::<EntryDate>().ok(),
            };
            input.date = date
                .filter(|date| *date <= today)
                .map(|date| date.to_string())
                .unwrap_or_default();
            if input.date.is_empty() {
                guidance.push("เลือกวันที่เอง ข้อความวันที่นี้ยังแปลงไม่ได้".into());
            }
        } else if [
            "วันจันทร์",
            "วันอังคาร",
            "วันพุธ",
            "วันพฤหัส",
            "วันศุกร์",
            "วันเสาร์",
            "วันอาทิตย์",
            "วันที่",
            "พรุ่งนี้",
            "มะรืน",
            "สัปดาห์",
            "เดือนก่อน",
            "เดือนที่แล้ว",
            "ปีที่แล้ว",
            "yesterday",
            "tomorrow",
            "/",
        ]
        .iter()
        .any(|word| source.contains(word))
        {
            input.date.clear();
            guidance.push("ข้อความมีวันที่ที่ยังแปลงไม่ได้ กรุณาเลือกวันที่เอง".into());
        } else {
            guidance.push("วันที่ตั้งต้นเป็นวันนี้ ตรวจว่าเป็นวันที่ทำรายการจริง".into());
        }
        input.note = candidate.description_text.unwrap_or_default();
        let draft = ModelDraft {
            input,
            guidance: guidance.join(" · "),
        };
        if !drafts.contains(&draft) {
            drafts.push(draft);
        }
    }
    Ok(QuickResolution::Choices {
        source: source.to_owned(),
        drafts,
    })
}
