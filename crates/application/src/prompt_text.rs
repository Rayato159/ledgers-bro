//! Local, bounded action grammar. A draft is never an instruction to write.
use crate::*;
use ledger_domain::*;

fn fail(message: &str) -> AppError {
    AppError::Input(message.into())
}
const VERBS: &[(&str, PromptKind)] = &[
    ("เพิ่มรายจ่ายประจำ", PromptKind::Recurring),
    ("เพิ่มหนี้รายเดือน", PromptKind::Recurring),
    ("ผูกรายจ่ายเดิม", PromptKind::LinkRecurring),
    ("ตั้งจำนวนงวด", PromptKind::CountRecurring),
    ("หยุดแผน", PromptKind::StopRecurring),
    ("จ่ายงวด", PromptKind::PayRecurring),
    ("เพิ่มบัญชี", PromptKind::Account),
    ("ลบบัญชี", PromptKind::DeleteAccount),
    ("ยกเลิกรายการ", PromptKind::Reverse),
    ("เพิ่มลูกหนี้", PromptKind::Receivable),
    ("ให้ยืมเงิน", PromptKind::Lending),
    ("รับชำระหนี้", PromptKind::Repayment),
];
fn action(text: &str) -> Option<(PromptKind, String)> {
    let text = text
        .trim()
        .trim_start_matches("วันนี้")
        .trim_start_matches("เมื่อวาน")
        .trim();
    for (verb, kind) in VERBS {
        if let Some(rest) = text.strip_prefix(verb) {
            return Some((*kind, rest.trim().into()));
        }
    }
    if let Some(rest) = text.strip_prefix("ให้")
        && let Some((name, tail)) = rest.split_once("ยืม")
        && !name.trim().is_empty()
    {
        return Some((
            PromptKind::Lending,
            format!("ชื่อ {} ยอด {}", name.trim(), tail.trim()),
        ));
    }
    for word in ["คืนหนี้", "ชำระหนี้", "ติดหนี้เรา"]
    {
        if let Some((name, tail)) = text.split_once(word)
            && !name.trim().is_empty()
        {
            return Some((
                if word == "ติดหนี้เรา" {
                    PromptKind::Receivable
                } else {
                    PromptKind::Repayment
                },
                format!("ชื่อ {} ยอด {}", name.trim(), tail.trim()),
            ));
        }
    }
    None
}
pub fn is_prompt_action(text: &str) -> bool {
    VERBS.iter().any(|(verb, _)| text.contains(verb))
        || ["คืนหนี้", "ชำระหนี้", "ติดหนี้เรา", "ให้ยืม", "งวด"]
            .iter()
            .any(|s| text.contains(s))
        || (text.contains("ให้") && text.contains("ยืม"))
}
fn split_actions(text: &str) -> Result<Vec<&str>, AppError> {
    let mut quoted = false;
    let mut start = 0;
    let mut result = vec![];
    for (i, ch) in text.char_indices() {
        if i < start {
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
            continue;
        }
        if quoted {
            continue;
        }
        if let Some(sep) = ["\n", ";", "แล้วก็", "และก็", "แล้ว", "และ"]
            .into_iter()
            .find(|s| text[i..].starts_with(s))
        {
            // Unquoted conjunctions always delimit actions. Unknown clauses fail
            // the entire interpretation rather than being folded into a name.
            result.push(text[start..i].trim());
            start = i + sep.len();
        }
    }
    if quoted {
        return Err(fail("เครื่องหมายคำพูดยังไม่ครบคู่ กรุณาปิดชื่อหรือรายละเอียดให้ครบ"));
    }
    result.push(text[start..].trim());
    if result.iter().any(|s| s.is_empty()) || result.len() > MAX_BATCH_ENTRIES {
        return Err(fail(
            "แยกคำสั่งด้วยขึ้นบรรทัดใหม่หรือ “แล้วก็” ครั้งละ 1–8 รายการ ชื่อที่มีคำว่า และ ให้ใส่เครื่องหมายคำพูด",
        ));
    }
    Ok(result)
}

pub fn resolve_prompt_text(
    source: &str,
    today: EntryDate,
    state: &LedgerState,
) -> Result<QuickResolution, AppError> {
    if is_receipt_prompt(source) {
        if state.currency != Currency::Thb {
            return Err(fail(
                "Receipt OCR currently supports THB ledgers only. Use manual entry in your ledger currency.",
            ));
        }
        return resolve_receipt_prompt(source, today, &state.accounts);
    }
    let normalized = normalize_prompt_currency(source, state.currency)?;
    let source = normalized.as_str();
    if !is_prompt_action(source) {
        return resolve_entry_text(source, today, &state.accounts);
    }
    // Cancellation is an explicit action in this grammar, not a negated purchase.
    check_model_source(&source.replace("ยกเลิกรายการ", "คำสั่งกลับรายการ"))?;
    if [
        "อย่า",
        "ห้าม",
        "ไม่ต้อง",
        "สมมติ",
        "สมมุติ",
        "หรือ",
        "ไม่ใช่",
        "พรุ่งนี้",
        "USD",
        "ดอลลาร์",
    ]
    .iter()
    .any(|s| source.contains(s))
    {
        return Err(fail(
            "ข้อความยังมีเงื่อนไข ปฏิเสธ วันในอนาคต หรือสกุลเงินอื่น กรุณาระบุคำสั่งที่ต้องการทำจริงเป็นเงินบาท แล้วตรวจยืนยันก่อนบันทึก",
        ));
    }
    let mut drafts = vec![];
    for clause in split_actions(source)? {
        let (kind, rest) = if let Some(action) = action(clause) {
            action
        } else {
            let body = clause
                .trim()
                .trim_start_matches("วันนี้")
                .trim_start_matches("เมื่อวาน")
                .trim();
            let verbs = [
                ("ได้รับเงินจาก", PromptKind::Income),
                ("ได้เงินจาก", PromptKind::Income),
                ("ได้รับเงิน", PromptKind::Income),
                ("ได้เงิน", PromptKind::Income),
                ("รับเงิน", PromptKind::Income),
                ("รับ", PromptKind::Income),
                ("ซื้อ", PromptKind::Expense),
                ("จ่าย", PromptKind::Expense),
                ("โอน", PromptKind::Transfer),
            ];
            let Some((verb, kind)) = verbs.iter().find(|(v, _)| body.starts_with(v)) else {
                return Err(fail(
                    "มีคำสั่งที่ยังอ่านไม่ได้ กรุณาใช้ตัวอย่างตามประเภทรายการหรือแยกคำสั่งให้ชัด ยังไม่มีรายการถูกบันทึก",
                ));
            };
            (*kind, body[verb.len()..].trim().to_owned())
        };
        let mut draft = parse_action(kind, &rest, today)?;
        draft.source = clause.to_owned();
        if clause.trim().starts_with("เมื่อวาน")
            && kind.fields().iter().any(|f| f.key == "date" && f.required)
        {
            draft.set("date", today.previous_day()?.to_string());
        }
        hydrate_prompt_draft(&mut draft, state, today);
        drafts.push(draft);
    }
    Ok(QuickResolution::Actions {
        source: source.into(),
        drafts,
    })
}
fn markers(kind: PromptKind) -> Vec<(&'static str, &'static str)> {
    let mut result = vec![];
    for field in kind.fields() {
        let labels: &[&str] = match field.key {
            "name" | "debtor" | "loan" | "plan" => &["ชื่อ"],
            "amount" => &["เงินต้น", "ยอด", "จำนวนเงิน"],
            "opening" => &["ยอดเริ่มต้น", "ยอดหนี้"],
            "account" => {
                if kind == PromptKind::DeleteAccount {
                    &["ชื่อ", "บัญชี"]
                } else {
                    &["จากบัญชี", "เข้าบัญชี", "บันทึกลง", "จาก", "เข้า", "บัญชี"]
                }
            }
            "destination" => &["ไป"],
            "category" => &["หมวด"],
            "date" => &["วันที่"],
            "start" => &["เริ่ม"],
            "month" => &["ตั้งแต่", "งวด"],
            "day" => &["เก็บทุกวันที่", "ทุกวันที่"],
            "count" => &["จำนวนงวด"],
            "description" => &["เรื่อง", "หนี้อะไร"],
            "note" => &["โน้ต", "รายละเอียด"],
            "interest" => &["ดอกเบี้ย"],
            "entry" => &["รายการ"],
            "kind" => &["ประเภท"],
            _ => &[],
        };
        result.extend(labels.iter().map(|s| (*s, field.key)));
    }
    result.sort_by_key(|(s, _)| std::cmp::Reverse(s.len()));
    result
}
fn parse_action(kind: PromptKind, text: &str, today: EntryDate) -> Result<PromptDraft, AppError> {
    let mut draft = PromptDraft::empty(kind, today);
    let markers = markers(kind);
    let mut quoted = false;
    let mut found = vec![];
    let mut skip = 0;
    for (i, ch) in text.char_indices() {
        if i < skip {
            continue;
        }
        if ch == '"' {
            quoted = !quoted;
        }
        // Require a boundary for labels. Names containing a label are quoted.
        if !quoted
            && (i == 0 || text[..i].chars().last().is_some_and(char::is_whitespace))
            && let Some((label, key)) = markers
                .iter()
                .find(|(label, _)| text[i..].starts_with(label))
        {
            found.push((i, i + label.len(), *key));
            skip = i + label.len();
        }
    }
    let head = text[..found.first().map_or(text.len(), |(i, _, _)| *i)].trim();
    if !head.is_empty() {
        parse_head(&mut draft, head)?;
    }
    let mut seen = std::collections::BTreeSet::new();
    for (index, (_, end, key)) in found.iter().enumerate() {
        if !seen.insert(*key) || (!draft.get(key).is_empty() && !["date", "start"].contains(key)) {
            return Err(fail("ระบุช่องเดียวกันหลายครั้ง กรุณาแยกแต่ละคำสั่งเป็นคนละบรรทัด"));
        }
        let value = text[*end..found.get(index + 1).map_or(text.len(), |(i, _, _)| *i)]
            .trim()
            .trim_matches('"')
            .trim();
        let value = match *key {
            "amount" | "opening" | "interest" => clean_money(value)?,
            "count" => value.trim_end_matches("งวด").trim().into(),
            "date" => match value {
                "วันนี้" => today.to_string(),
                "เมื่อวาน" => today.previous_day()?.to_string(),
                _ => value.into(),
            },
            _ => value.into(),
        };
        draft.set(key, value);
    }
    Ok(draft)
}
fn clean_money(value: &str) -> Result<String, AppError> {
    let value = value.trim_end_matches("บาท").trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    let whole = value.split('.').next().unwrap_or_default();
    if whole.contains(',') {
        let groups: Vec<_> = whole.split(',').collect();
        if groups[0].is_empty() || groups[0].len() > 3 || groups[1..].iter().any(|s| s.len() != 3) {
            return Err(fail("รูปแบบยอดไม่ชัดเจน ใช้ 1000 หรือ 1,000.00 บาท"));
        }
    }
    if value.split_once('.').is_some_and(|(_, f)| f.contains(',')) {
        return Err(fail("ทศนิยมต้องไม่มีลูกน้ำ"));
    }
    let value = value.replace(',', "");
    value.parse::<Money>()?;
    Ok(value)
}
fn parse_head(draft: &mut PromptDraft, head: &str) -> Result<(), AppError> {
    let key = match draft.kind {
        PromptKind::Account | PromptKind::Recurring => "name",
        PromptKind::DeleteAccount => "account",
        PromptKind::PayRecurring
        | PromptKind::LinkRecurring
        | PromptKind::CountRecurring
        | PromptKind::StopRecurring => "plan",
        PromptKind::Receivable | PromptKind::Lending => "debtor",
        PromptKind::Repayment => "loan",
        PromptKind::Reverse => "entry",
        _ => "note",
    };
    let takes_amount =
        draft.kind.fields().iter().any(|f| f.key == "amount") && draft.kind != PromptKind::Reverse;
    if takes_amount && let Some((i, _)) = head.char_indices().find(|(_, ch)| ch.is_ascii_digit()) {
        // Only an amount at the END of the unlabelled head is accepted. Any
        // additional number/prose is an error, never silently discarded.
        draft.set("amount", clean_money(&head[i..])?);
        let name = head[..i]
            .trim()
            .trim_end_matches("ไป")
            .trim()
            .trim_matches('"');
        if name.ends_with(['-', '+']) {
            return Err(fail("กรุณาระบุยอดเงินบวกโดยไม่ใส่เครื่องหมาย"));
        }
        if !name.is_empty() {
            draft.set(key, name.into());
        }
    } else {
        draft.set(key, head.trim_matches('"').into());
    }
    Ok(())
}
