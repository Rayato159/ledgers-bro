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
    // Only a terminal list of account names is a global assignment. Split
    // transaction clauses first when each clause supplies its own account.
    let terminal = unquoted_assignment(source);
    let global = if let Some((body, tail)) = terminal {
        let names = split_list(tail.trim().trim_end_matches("ตามลำดับ"))?;
        (tail.trim().ends_with("ตามลำดับ")
            || (names.len() > 1 && !names.iter().any(|name| starts_transaction(name))))
        .then_some((body, tail))
    } else {
        None
    };
    let clauses = split_list(global.map_or(source, |(body, _)| body))?;
    if clauses.is_empty()
        || clauses.len() > MAX_BATCH_ENTRIES
        || clauses.iter().any(|p| p.is_empty())
    {
        return Err(AppError::Input(
            "แบ่งข้อความด้วย “และ”, “แล้วก็” หรือขึ้นบรรทัดใหม่ ครั้งละไม่เกิน 8 รายการ".into(),
        ));
    }
    let mut drafts = Vec::new();
    for clause in clauses {
        let (body, assignment) = unquoted_assignment(clause)
            .map_or((clause, None), |(body, tail)| {
                (body.trim(), Some(tail.trim()))
            });
        let mut draft = if let Ok(QuickResolution::Draft { input, guidance }) =
            resolve_quick_entry(body, today, accounts)
        {
            ModelDraft { input, guidance }
        } else {
            parse_clause(body, today, accounts)?
        };
        if let Some(name) = assignment {
            if draft.input.account.is_none() {
                // Account, category and trailing date all belong to this clause.
                let canonical = format!("{body} จากบัญชี{name}");
                draft = if let Ok(QuickResolution::Draft { input, guidance }) =
                    resolve_quick_entry(&canonical, today, accounts)
                {
                    ModelDraft { input, guidance }
                } else {
                    parse_clause(&canonical, today, accounts)?
                };
            } else {
                assign_account(&mut draft, name, accounts)?;
            }
        }
        drafts.push(draft);
    }
    if let Some((_, assignment)) = global {
        let ordered = assignment.trim().strip_suffix("ตามลำดับ");
        let names = split_list(ordered.unwrap_or(assignment))?;
        let unambiguous = names.len() == drafts.len() && (drafts.len() == 1 || ordered.is_some());
        for (index, draft) in drafts.iter_mut().enumerate() {
            if unambiguous {
                assign_account(draft, names[index], accounts)?;
            } else {
                draft.input.account = None;
                draft.guidance =
                    "จำนวนบัญชีไม่ตรงกับรายการ หรือยังไม่ได้ระบุ “ตามลำดับ” กรุณาเลือกบัญชีให้แต่ละรายการ".into();
            }
        }
    }
    Ok(QuickResolution::Batch {
        source: source.to_owned(),
        drafts,
    })
}

fn unquoted_assignment(text: &str) -> Option<(&str, &str)> {
    let mut quoted = false;
    for (index, ch) in text.char_indices() {
        if ch == '"' {
            quoted = !quoted;
        }
        if !quoted && text[index..].starts_with("บันทึกลง") {
            return Some((&text[..index], &text[index + "บันทึกลง".len()..]));
        }
    }
    None
}

fn starts_transaction(text: &str) -> bool {
    let text = text
        .trim()
        .trim_start_matches("วันนี้")
        .trim_start_matches("เมื่อวาน")
        .trim();
    ["ซื้อ", "จ่าย", "รับ", "ได้เงิน", "ได้รับ", "โอน"]
        .iter()
        .any(|word| text.starts_with(word))
}

fn assign_account(
    draft: &mut ModelDraft,
    name: &str,
    accounts: &[Account],
) -> Result<(), AppError> {
    let found = find_account(name, accounts);
    if found.is_none() && (name.contains("บันทึกลง") || name.contains(|c: char| c.is_ascii_digit()))
    {
        return Err(AppError::Input(
            "ยังมีข้อความหรือยอดเงินหลังชื่อบัญชีที่อ่านไม่ครบ กรุณาแยกรายการด้วย “แล้วก็” หรือขึ้นบรรทัดใหม่".into(),
        ));
    }
    if draft.input.account.is_some() && draft.input.account != found {
        draft.input.account = None;
        draft.guidance = "บัญชีในรายการขัดกับบัญชีท้ายข้อความ กรุณาเลือกบัญชีที่ต้องการ".into();
    } else {
        draft.input.account = found;
        draft.guidance = if found.is_none() {
            format!(
                "ไม่พบบัญชี “{}” กรุณาเพิ่มบัญชีนี้ในหน้าบัญชี หรือเลือกบัญชีที่มีอยู่",
                name.trim()
            )
        } else {
            String::new()
        };
    }
    Ok(())
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
        } else if !quoted
            && let Some(separator) = ["แล้วก็", "และก็", "แล้ว", "และ", "\n"]
                .iter()
                .find(|s| text[index..].starts_with(**s))
        {
            parts.push(text[start..index].trim());
            start = index + separator.len();
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
    if input
        .recurring
        .as_ref()
        .is_some_and(|s| s.month.parse::<ledger_domain::Month>().is_err())
    {
        fields.push("งวดเดือนที่ต้องการจ่าย (YYYY-MM)");
    }
    if input.date.parse::<EntryDate>().is_err() {
        fields.push("วันที่รายการ");
    }
    if let Some(receipt) = &input.receipt {
        if receipt.reconcile(&input.amount).is_err() {
            fields.push("รายละเอียดใบเสร็จและยอดรวมที่ตรงกัน");
        }
        if !receipt.reviewed {
            fields.push("ยืนยันตรวจรายละเอียดกับภาพใบเสร็จ");
        }
    }
    fields
}
