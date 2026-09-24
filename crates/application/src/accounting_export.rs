use crate::Dashboard;
use ledger_domain::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ExportLanguage {
    #[default]
    Thai,
    English,
}
impl ExportLanguage {
    fn text(self, thai: &str, english: &str) -> String {
        match self {
            Self::Thai => thai,
            Self::English => english,
        }
        .to_owned()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AccountingReport {
    #[default]
    Journal,
    GeneralLedger,
    TrialBalance,
}
impl AccountingReport {
    pub const ALL: [Self; 3] = [Self::Journal, Self::GeneralLedger, Self::TrialBalance];
    pub const fn code(self) -> &'static str {
        match self {
            Self::Journal => "journal",
            Self::GeneralLedger => "ledger",
            Self::TrialBalance => "trial-balance",
        }
    }
    pub fn label(self, language: ExportLanguage) -> String {
        match self {
            Self::Journal => language.text("สมุดรายวันทั่วไป", "General journal"),
            Self::GeneralLedger => language.text("บัญชีแยกประเภท", "General ledger"),
            Self::TrialBalance => language.text("งบทดลอง", "Trial balance"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportOptions {
    pub language: ExportLanguage,
    pub report: AccountingReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvExport {
    pub filename: String,
    pub contents: String,
}

struct JournalLine {
    date: EntryDate,
    entry: EntryId,
    kind: String,
    status: String,
    original: String,
    account_id: String,
    account_name: String,
    signed_minor: i128,
    note: String,
}

/// Exports the actual balanced postings, including both originals and reversals.
/// Positive posting = debit; negative posting = credit, regardless of account kind.
/// Reports cover all recorded entries up to `today`, not only the dashboard month.
/// This does not infer VAT, withholding, accruals, or legal tax eligibility.
pub fn export_csv(view: &Dashboard, options: ExportOptions) -> Result<CsvExport, DomainError> {
    let lang = options.language;
    let mut entries: Vec<_> = view
        .entries
        .iter()
        .rev()
        .filter(|e| e.date() <= view.today)
        .collect();
    entries.sort_by_key(|e| e.date()); // Stable: preserve recording order within a day.
    let lookup: BTreeMap<_, _> = view.entries.iter().map(|e| (e.id(), e)).collect();
    let mut lines = Vec::new();
    for entry in entries {
        let original = match entry.kind() {
            EntryKind::Reversal { original } => {
                Some(*lookup.get(original).ok_or(DomainError::InvalidReversal)?)
            }
            _ => None,
        };
        let underlying = original.unwrap_or(entry);
        let kind = match entry.kind() {
            EntryKind::ReceivableOpening { .. } => lang.text("ยอดลูกหนี้ยกมา", "Opening receivable"),
            EntryKind::Lending { .. } => lang.text("ให้ยืมเงิน", "Loan advance"),
            EntryKind::Repayment { .. } => lang.text("ลูกหนี้ชำระเงินต้น", "Principal repayment"),
            EntryKind::Opening { .. } => lang.text("ยอดเริ่มต้น", "Opening balance"),
            EntryKind::Income { .. } => lang.text("รายรับ", "Income"),
            EntryKind::Expense { .. } => lang.text("รายจ่าย", "Expense"),
            EntryKind::Transfer { .. } => lang.text("โอนเงิน", "Transfer"),
            EntryKind::Reversal { .. } => lang.text("กลับรายการ", "Reversal"),
        };
        let status = if original.is_some() {
            lang.text("รายการกลับบัญชี", "Reversal entry")
        } else if view.reversed.contains(&entry.id()) {
            lang.text("ถูกกลับรายการแล้ว", "Reversed")
        } else {
            lang.text("บันทึกแล้ว", "Posted")
        };
        let sum: i128 = entry
            .postings()
            .iter()
            .map(|p| i128::from(p.amount().minor()))
            .sum();
        if sum != 0 {
            return Err(DomainError::UnbalancedJournal);
        }
        let mut postings: Vec<_> = entry.postings().iter().collect();
        postings.sort_by_key(|p| p.amount().minor() < 0); // Debit lines precede credits.
        for posting in postings {
            let (account_id, account_name) = match posting.target() {
                PostingTarget::System(SystemBook::Receivable) => {
                    let id = match underlying.kind() {
                        EntryKind::ReceivableOpening { receivable, .. }
                        | EntryKind::Lending { receivable, .. }
                        | EntryKind::Repayment { receivable, .. } => *receivable,
                        _ => return Err(DomainError::InvalidReceivable),
                    };
                    let loan = view
                        .receivables
                        .iter()
                        .find(|r| r.id() == id)
                        .ok_or(DomainError::InvalidReceivable)?;
                    (
                        format!("receivable:{id}"),
                        format!(
                            "{} — {} · {}",
                            lang.text("ลูกหนี้", "Receivable"),
                            loan.debtor().as_str(),
                            loan.description().as_str()
                        ),
                    )
                }
                PostingTarget::Account(id) => {
                    let account = view
                        .accounts
                        .iter()
                        .find(|a| a.account.id() == id)
                        .ok_or(DomainError::AccountUnavailable)?;
                    (id.to_string(), account.account.name().as_str().to_owned())
                }
                PostingTarget::System(SystemBook::Equity) => (
                    "equity:opening".into(),
                    lang.text("ส่วนทุน — ยอดเริ่มต้น", "Equity — opening balance"),
                ),
                PostingTarget::System(SystemBook::Income) => {
                    let EntryKind::Income { category, .. } = underlying.kind() else {
                        return Err(DomainError::InvalidCategory);
                    };
                    (
                        format!("income:{}", category.code()),
                        category_label(*category, lang),
                    )
                }
                PostingTarget::System(SystemBook::Expense) => {
                    let EntryKind::Expense { category, .. } = underlying.kind() else {
                        return Err(DomainError::InvalidCategory);
                    };
                    (
                        format!("expense:{}", category.code()),
                        category_label(*category, lang),
                    )
                }
            };
            let note = match entry.kind() {
                EntryKind::Opening { .. } if entry.note().as_str() == "ยอดเริ่มต้น" => {
                    kind.clone()
                }
                EntryKind::Reversal { .. } if entry.note().as_str() == "ยกเลิกรายการโดยผู้ใช้" => {
                    lang.text("ยกเลิกรายการโดยผู้ใช้", "Cancelled by user")
                }
                _ => entry.note().as_str().to_owned(),
            };
            lines.push(JournalLine {
                date: entry.date(),
                entry: entry.id(),
                kind: kind.clone(),
                status: status.clone(),
                original: original.map_or_else(String::new, |e| e.id().to_string()),
                account_id,
                account_name,
                signed_minor: i128::from(posting.amount().minor()),
                note,
            });
        }
    }
    let contents = match options.report {
        AccountingReport::Journal => journal_csv(&lines, lang, false, view.currency)?,
        AccountingReport::GeneralLedger => {
            lines.sort_by(|a, b| a.account_id.cmp(&b.account_id));
            journal_csv(&lines, lang, true, view.currency)?
        }
        AccountingReport::TrialBalance => {
            trial_balance_csv(&lines, lang, view.today, view.currency)?
        }
    };
    Ok(CsvExport {
        filename: format!("{}-{}.csv", options.report.label(lang), view.today),
        contents,
    })
}

fn category_label(category: Category, language: ExportLanguage) -> String {
    let english = match category {
        Category::Rent => "Rent expense",
        Category::Food => "Food expense",
        Category::Snacks => "Snacks expense",
        Category::Luxury => "Discretionary expense",
        Category::Supplies => "Supplies expense",
        Category::Medical => "Medical expense",
        Category::OtherExpense => "Other expense",
        Category::Salary => "Salary income",
        Category::Freelance => "Freelance income",
        Category::Interest => "Interest income",
        Category::OtherIncome => "Other income",
    };
    language.text(
        &format!(
            "{} — {}",
            if category.is_income() {
                "รายได้"
            } else {
                "ค่าใช้จ่าย"
            },
            category.label()
        ),
        english,
    )
}

fn journal_csv(
    lines: &[JournalLine],
    lang: ExportLanguage,
    ledger: bool,
    currency: Currency,
) -> Result<String, DomainError> {
    let mut csv = String::from("\u{feff}");
    let mut headers = [
        ("วันที่ (ค.ศ.)", "Date (CE)"),
        ("รหัสรายการ", "Entry ID"),
        ("ประเภท", "Type"),
        ("สถานะ", "Status"),
        ("อ้างอิงรายการเดิม", "Original entry ID"),
        ("รหัสบัญชี", "Account ID"),
        ("ชื่อบัญชี", "Account name"),
        ("เดบิต (บาท)", "Debit (THB)"),
        ("เครดิต (บาท)", "Credit (THB)"),
        ("คำอธิบาย", "Description"),
    ]
    .map(|(th, en)| lang.text(th, en))
    .to_vec();
    if ledger {
        headers.extend([
            lang.text("ยอดคงเหลือเดบิต (บาท)", "Debit balance (THB)"),
            lang.text("ยอดคงเหลือเครดิต (บาท)", "Credit balance (THB)"),
        ]);
    }
    for header in &mut headers {
        *header = currency_header(header, currency);
    }
    write_row(&mut csv, &headers);
    let mut totals = (0_i128, 0_i128);
    let mut balances = BTreeMap::<&str, i128>::new();
    for line in lines {
        let (debit, credit) = sides(line.signed_minor);
        totals.0 += debit;
        totals.1 += credit;
        let mut fields = vec![
            line.date.to_string(),
            line.entry.to_string(),
            line.kind.clone(),
            line.status.clone(),
            line.original.clone(),
            line.account_id.clone(),
            line.account_name.clone(),
            decimal(debit),
            decimal(credit),
            line.note.clone(),
        ];
        if ledger {
            let balance = balances.entry(&line.account_id).or_default();
            *balance += line.signed_minor;
            let (dr, cr) = sides(*balance);
            fields.extend([decimal(dr), decimal(cr)]);
        }
        write_row(&mut csv, &fields);
    }
    if totals.0 != totals.1 {
        return Err(DomainError::UnbalancedJournal);
    }
    let mut total = vec![String::new(); headers.len()];
    total[6] = lang.text("รวม", "Total");
    total[7] = decimal(totals.0);
    total[8] = decimal(totals.1);
    write_row(&mut csv, &total);
    Ok(csv)
}

fn trial_balance_csv(
    lines: &[JournalLine],
    lang: ExportLanguage,
    as_of: EntryDate,
    currency: Currency,
) -> Result<String, DomainError> {
    let mut csv = String::from("\u{feff}");
    write_row(
        &mut csv,
        &[
            ("ณ วันที่ (ค.ศ.)", "As of (CE)"),
            ("รหัสบัญชี", "Account ID"),
            ("ชื่อบัญชี", "Account name"),
            ("ยอดรวมเดบิต (บาท)", "Debit turnover (THB)"),
            ("ยอดรวมเครดิต (บาท)", "Credit turnover (THB)"),
            ("ยอดคงเหลือเดบิต (บาท)", "Debit balance (THB)"),
            ("ยอดคงเหลือเครดิต (บาท)", "Credit balance (THB)"),
        ]
        .map(|(th, en)| currency_header(&lang.text(th, en), currency)),
    );
    let mut accounts = BTreeMap::<&str, (&str, i128, i128)>::new();
    for line in lines {
        let account = accounts
            .entry(&line.account_id)
            .or_insert((&line.account_name, 0, 0));
        let (dr, cr) = sides(line.signed_minor);
        account.1 += dr;
        account.2 += cr;
    }
    let mut totals = [0_i128; 4];
    for (id, (name, dr, cr)) in accounts {
        let (balance_dr, balance_cr) = sides(dr - cr);
        for (total, value) in totals.iter_mut().zip([dr, cr, balance_dr, balance_cr]) {
            *total += value;
        }
        write_row(
            &mut csv,
            &[
                as_of.to_string(),
                id.into(),
                name.into(),
                decimal(dr),
                decimal(cr),
                decimal(balance_dr),
                decimal(balance_cr),
            ],
        );
    }
    if totals[0] != totals[1] || totals[2] != totals[3] {
        return Err(DomainError::UnbalancedJournal);
    }
    write_row(
        &mut csv,
        &[
            as_of.to_string(),
            String::new(),
            lang.text("รวม", "Total"),
            decimal(totals[0]),
            decimal(totals[1]),
            decimal(totals[2]),
            decimal(totals[3]),
        ],
    );
    Ok(csv)
}

fn sides(value: i128) -> (i128, i128) {
    (value.max(0), (-value).max(0))
}
// Activity totals can exceed the per-balance Money limit; keep exact integer satang.
fn decimal(minor: i128) -> String {
    format!("{}.{:02}", minor / 100, minor % 100)
}

/// RFC 4180 escaping and formula neutralization, with UTF-8 BOM for Thai in Excel.
fn write_row(csv: &mut String, cells: &[String]) {
    for (index, value) in cells.iter().enumerate() {
        if index > 0 {
            csv.push(',');
        }
        csv.push('"');
        if value.trim_start().starts_with(['=', '+', '-', '@']) {
            csv.push('\'');
        }
        csv.push_str(&value.replace('"', "\"\""));
        csv.push('"');
    }
    csv.push_str("\r\n");
}

fn currency_header(label: &str, currency: Currency) -> String {
    if currency == Currency::Thb {
        label.to_owned()
    } else {
        label
            .replace("(THB)", &format!("({})", currency.code()))
            .replace("(บาท)", &format!("({})", currency.code()))
    }
}
