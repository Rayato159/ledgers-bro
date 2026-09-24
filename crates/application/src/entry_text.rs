//! Bounded Thai phrases. This parser never writes and never invents categories,
//! accounts, numbers or a second interpretation. Unrecognised prose can use AI.
use crate::{
    AppError, EntryInput, ModelDraft, QuickResolution, TransactionKind, check_model_source,
    resolve_quick_entry,
};
use ledger_domain::{Account, Category, EntryDate, PositiveMoney};

pub const MAX_BATCH_ENTRIES: usize = 8;

pub fn resolve_entry_text(
    source: &str,
    today: EntryDate,
    accounts: &[Account],
) -> Result<QuickResolution, AppError> {
    if let Ok(resolution) = resolve_quick_entry(source, today, accounts) {
        return Ok(resolution);
    }
    check_model_source(source)?;
    if [
        "vat",
        "withholding",
        "หัก ณ",
        "หักณ",
        "ภาษี",
        "ก่อนหัก",
        "หลังหัก",
        "หรือ",
        "ไม่ใช่",
    ]
    .iter()
    .any(|word| source.to_lowercase().contains(word))
    {
        return Err(AppError::Input(
            "ข้อความมีหลายความหมายหรือรายละเอียดภาษี กรุณาระบุยอดรับจ่ายจริงให้ชัด แยกแต่ละรายการก่อนบันทึก".into(),
        ));
    }
    let (body, assignment) = source
        .split_once("บันทึกลง")
        .map_or((source.trim(), None), |(body, tail)| {
            (body.trim(), Some(tail.trim()))
        });
    let clauses = split_list(body)?;
    if clauses.is_empty()
        || clauses.len() > MAX_BATCH_ENTRIES
        || clauses.iter().any(|part| part.is_empty())
    {
        return Err(AppError::Input(
            "แบ่งข้อความเป็นรายการชัดเจนด้วยคำว่า “และ” หรือขึ้นบรรทัดใหม่ ครั้งละไม่เกิน 8 รายการ".into(),
        ));
    }
    let mut drafts = Vec::new();
    for clause in &clauses {
        if let Ok(QuickResolution::Draft { input, guidance }) =
            resolve_quick_entry(clause, today, accounts)
        {
            drafts.push(ModelDraft { input, guidance });
        } else {
            drafts.push(parse_clause(clause, today, accounts)?);
        }
    }
    if let Some(assignment) = assignment {
        let ordered = assignment.strip_suffix("ตามลำดับ");
        let names: Vec<_> = split_list(ordered.unwrap_or(assignment))?
            .into_iter()
            .map(|s| s.trim().trim_matches('"'))
            .collect();
        let unambiguous = names.len() == drafts.len() && (drafts.len() == 1 || ordered.is_some());
        for (index, draft) in drafts.iter_mut().enumerate() {
            // A global assignment conflicting with a clause must be reviewed,
            // never silently replace the original account.
            let found = if unambiguous {
                find_account(names[index], accounts)
            } else {
                None
            };
            if draft.input.account.is_some() && draft.input.account != found {
                draft.input.account = None;
                draft.guidance = "บัญชีในรายการขัดกับบัญชีท้ายข้อความ กรุณาเลือกบัญชีที่ต้องการ".into();
            } else {
                draft.input.account = found;
                draft.guidance = if !unambiguous {
                    "จำนวนบัญชีไม่ตรงกับรายการ หรือยังไม่ได้ระบุ “ตามลำดับ” กรุณาเลือกบัญชีให้แต่ละรายการ".into()
                } else if found.is_none() {
                    format!(
                        "ไม่พบบัญชี “{}” กรุณาเพิ่มบัญชีนี้ในหน้าบัญชี หรือเลือกบัญชีที่มีอยู่",
                        names[index]
                    )
                } else {
                    String::new()
                };
            }
        }
    }
    Ok(QuickResolution::Batch {
        source: source.to_owned(),
        drafts,
    })
}

// Quoted account names and notes may themselves contain “และ”.
fn split_list(text: &str) -> Result<Vec<&str>, AppError> {
    let mut quoted = false;
    let mut start = 0;
    let mut parts = Vec::new();
    for (index, ch) in text.char_indices() {
        if index < start {
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
        } else if !quoted && (ch == '\n' || text[index..].starts_with("และ")) {
            parts.push(text[start..index].trim());
            start = index + if ch == '\n' { 1 } else { "และ".len() };
        }
    }
    if quoted {
        return Err(AppError::Input(
            "เครื่องหมายคำพูดยังไม่ครบคู่ กรุณาปิดชื่อบัญชีหรือรายละเอียดให้ครบ".into(),
        ));
    }
    parts.push(text[start..].trim());
    Ok(parts)
}

fn find_account(name: &str, accounts: &[Account]) -> Option<ledger_domain::AccountId> {
    let name = name.trim().trim_matches('"').to_lowercase();
    accounts
        .iter()
        .find(|a| !a.is_archived() && a.name().key() == name)
        .map(Account::id)
}

fn parse_clause(
    clause: &str,
    today: EntryDate,
    accounts: &[Account],
) -> Result<ModelDraft, AppError> {
    let mut input = EntryInput::empty(today);
    let mut clause = clause.trim();
    if clause.contains("วันนี้") && clause.contains("เมื่อวาน") {
        return Err(AppError::Input(
            "รายการเดียวระบุทั้งวันนี้และเมื่อวาน กรุณาเลือกวันที่เดียวหรือแยกเป็นสองรายการ".into(),
        ));
    }
    for word in ["เมื่อวาน", "วันนี้"] {
        let date = if word == "เมื่อวาน" && (clause.starts_with(word) || clause.ends_with(word))
        {
            today.previous_day()?
        } else {
            today
        };
        if let Some(rest) = clause.strip_prefix(word) {
            input.date = date.to_string();
            clause = rest.trim();
        }
        if let Some(rest) = clause.strip_suffix(word) {
            input.date = date.to_string();
            clause = rest.trim();
        }
    }
    let verbs = [
        ("ได้เงินจาก", TransactionKind::Income),
        ("ได้รับเงินจาก", TransactionKind::Income),
        ("ได้รับเงิน", TransactionKind::Income),
        ("ได้เงิน", TransactionKind::Income),
        ("รับเงิน", TransactionKind::Income),
        ("ซื้อ", TransactionKind::Expense),
        ("จ่ายค่า", TransactionKind::Expense),
        ("จ่าย", TransactionKind::Expense),
    ];
    let Some((verb, kind)) = verbs.iter().find(|(verb, _)| clause.starts_with(verb)) else {
        return Err(AppError::Input("ยังแยกข้อความนี้ไม่ได้ ลอง “ซื้อไก่ทอด 30 บาท และได้เงินจาก Facebook 400 บาท บันทึกลง เงินสด และ กรุงไทย ตามลำดับ” หรือกรอกเอง".into()));
    };
    input.kind = *kind;
    let rest = clause[verb.len()..].trim();
    let number_start = rest
        .char_indices()
        .find(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i);
    let (description, tail) = if let Some(start) = number_start {
        let end = rest[start..]
            .char_indices()
            .find(|(_, c)| !c.is_ascii_digit() && *c != '.' && *c != ',')
            .map_or(rest.len(), |(i, _)| start + i);
        if rest[..start].trim_end().ends_with(['-', '+']) {
            return Err(AppError::Input(
                "รายการหนึ่งมีหลายยอดหรือรูปแบบยอดไม่ชัดเจน กรุณาแยกด้วย “และ” และใช้ยอดบาทหนึ่งยอดต่อรายการ"
                    .into(),
            ));
        }
        let amount = &rest[start..end];
        let whole = amount.split('.').next().unwrap_or_default();
        if whole.contains(',') {
            let groups: Vec<_> = whole.split(',').collect();
            if groups[0].is_empty()
                || groups[0].len() > 3
                || groups[1..].iter().any(|g| g.len() != 3)
            {
                return Err(AppError::Input(
                    "คั่นหลักพันด้วยลูกน้ำ เช่น 1,000.50 บาท หรือใช้ 1000.50".into(),
                ));
            }
        }
        if amount
            .split_once('.')
            .is_some_and(|(_, fraction)| fraction.contains(','))
        {
            return Err(AppError::Input("ยอดทศนิยมต้องไม่มีลูกน้ำ".into()));
        }
        input.amount = PositiveMoney::new(amount.replace(',', "").parse()?)?
            .money()
            .to_string();
        (
            &rest[..start],
            rest[end..]
                .trim()
                .strip_prefix("บาท")
                .unwrap_or(rest[end..].trim())
                .trim(),
        )
    } else {
        (rest, "")
    };
    input.note = description.trim().trim_end_matches("ไป").trim().to_owned();
    // Preserve only explicit category names. Product names are not a tax or category rule.
    let (tail, category) = tail
        .split_once("หมวด")
        .map_or((tail, None), |(a, b)| (a.trim(), Some(b.trim())));
    if let Some(category) = category {
        if category.contains(|ch: char| ch.is_ascii_digit()) {
            return Err(AppError::Input(
                "พบตัวเลขเพิ่มเติมในหมวดหมู่ กรุณาแยกยอดและหมวดของแต่ละรายการให้ชัดเจน".into(),
            ));
        }
        if category == "หนี้" {
            return Err(ledger_domain::DomainError::DebtNeedsTransfer.into());
        }
        let allowed = if input.kind == TransactionKind::Income {
            &Category::INCOME[..]
        } else {
            &Category::EXPENSE[..]
        };
        input.category = allowed.iter().copied().find(|c| c.label() == category);
    }
    let account_name = ["จ่ายจาก", "จากบัญชี", "เข้าบัญชี", "จาก", "เข้า", "จ่าย"]
        .iter()
        .find_map(|prefix| tail.strip_prefix(prefix))
        .map(str::trim);
    if !tail.is_empty() && account_name.is_none() {
        return Err(AppError::Input("มีข้อความท้ายรายการที่ยังอ่านไม่ครบ กรุณาใช้ “จาก ชื่อบัญชี” หรือ “เข้า ชื่อบัญชี” และระบุวันที่ด้วย วันนี้/เมื่อวาน".into()));
    }
    input.account = account_name.and_then(|name| find_account(name, accounts));
    if tail.contains(|ch: char| ch.is_ascii_digit()) && input.account.is_none() {
        return Err(AppError::Input(
            "พบตัวเลขเพิ่มเติมที่ยังระบุความหมายไม่ได้ กรุณาแยกรายการและยอดให้ชัดเจน".into(),
        ));
    }
    let guidance = match account_name {
        Some(name) if input.account.is_none() => {
            format!("ไม่พบบัญชี “{name}” กรุณาเพิ่มบัญชีหรือเลือกบัญชีที่มีอยู่")
        }
        _ => String::new(),
    };
    Ok(ModelDraft { input, guidance })
}

/// The same checklist drives both assistant feedback and the disabled save state.
pub fn missing_entry_fields(input: &EntryInput) -> Vec<&'static str> {
    let mut fields = Vec::new();
    if input
        .amount
        .parse()
        .ok()
        .and_then(|v| PositiveMoney::new(v).ok())
        .is_none()
    {
        fields.push("ยอดเงินที่มากกว่า 0");
    }
    if input.account.is_none() {
        fields.push("บัญชีที่ใช้รับหรือจ่าย");
    }
    if input.kind == TransactionKind::Transfer {
        if input.destination.is_none() {
            fields.push("บัญชีปลายทาง");
        }
    } else if input.category.is_none() {
        fields.push("หมวดหมู่");
    }
    if input.date.parse::<EntryDate>().is_err() {
        fields.push("วันที่รายการ");
    }
    fields
}
