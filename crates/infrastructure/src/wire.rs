use ledger_application::StorageError;
use ledger_domain::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Persistence DTOs are deliberately separate from domain types. Deserialization
// cannot bypass Money, Note, ID, date, category, or journal constructors.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum StoredKind {
    ReceivableOpening {
        receivable: String,
        amount: i64,
    },
    Lending {
        receivable: String,
        account: String,
        amount: i64,
    },
    Repayment {
        receivable: String,
        account: String,
        amount: i64,
    },
    Opening {
        account: String,
        balance: i64,
    },
    Expense {
        account: String,
        amount: i64,
        category: String,
    },
    Income {
        account: String,
        amount: i64,
        category: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tax: Option<StoredIncomeTax>,
    },
    Transfer {
        from: String,
        to: String,
        amount: i64,
    },
    Reversal {
        original: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredIncomeTax {
    section: u8,
    gross: i64,
    withholding: i64,
    vat: i64,
    other_deductions: i64,
}
impl StoredIncomeTax {
    fn from_domain(tax: IncomeTax) -> Self {
        Self {
            section: tax.section().number(),
            gross: tax.gross().minor(),
            withholding: tax.withholding().minor(),
            vat: tax.vat().minor(),
            other_deductions: tax.other_deductions().minor(),
        }
    }
    fn restore(self) -> Result<IncomeTax, DomainError> {
        IncomeTax::new(
            IncomeSection::new(self.section)?,
            PositiveMoney::new(Money::from_minor(self.gross)?)?,
            Money::from_minor(self.withholding)?,
            Money::from_minor(self.vat)?,
            Money::from_minor(self.other_deductions)?,
        )
    }
}

impl StoredKind {
    pub fn from_entry(entry: &JournalEntry) -> Self {
        match *entry.kind() {
            EntryKind::ReceivableOpening { receivable, amount } => Self::ReceivableOpening {
                receivable: receivable.to_string(),
                amount: amount.money().minor(),
            },
            EntryKind::Lending {
                receivable,
                account,
                amount,
            } => Self::Lending {
                receivable: receivable.to_string(),
                account: account.to_string(),
                amount: amount.money().minor(),
            },
            EntryKind::Repayment {
                receivable,
                account,
                amount,
            } => Self::Repayment {
                receivable: receivable.to_string(),
                account: account.to_string(),
                amount: amount.money().minor(),
            },
            EntryKind::Opening { account, balance } => Self::Opening {
                account: account.to_string(),
                balance: balance.minor(),
            },
            EntryKind::Expense {
                account,
                amount,
                category,
            } => Self::Expense {
                account: account.to_string(),
                amount: amount.money().minor(),
                category: category.code().into(),
            },
            EntryKind::Income {
                account,
                amount,
                category,
            } => Self::Income {
                account: account.to_string(),
                amount: amount.money().minor(),
                category: category.code().into(),
                tax: entry.income_tax().map(StoredIncomeTax::from_domain),
            },
            EntryKind::Transfer { from, to, amount } => Self::Transfer {
                from: from.to_string(),
                to: to.to_string(),
                amount: amount.money().minor(),
            },
            EntryKind::Reversal { original } => Self::Reversal {
                original: original.to_string(),
            },
        }
    }
    pub fn encode(entry: &JournalEntry) -> Result<String, StorageError> {
        serde_json::to_string(&Self::from_entry(entry)).map_err(|_| StorageError::Corrupt)
    }
    pub fn restore(
        self,
        id: EntryId,
        date: EntryDate,
        note: Note,
        previous: &BTreeMap<EntryId, JournalEntry>,
    ) -> Result<JournalEntry, StorageError> {
        let amount = |n| PositiveMoney::new(Money::from_minor(n)?);
        let mut income_tax = None;
        let kind = match self {
            Self::ReceivableOpening {
                receivable,
                amount: n,
            } => EntryKind::ReceivableOpening {
                receivable: receivable.parse()?,
                amount: amount(n)?,
            },
            Self::Lending {
                receivable,
                account,
                amount: n,
            } => EntryKind::Lending {
                receivable: receivable.parse()?,
                account: account.parse()?,
                amount: amount(n)?,
            },
            Self::Repayment {
                receivable,
                account,
                amount: n,
            } => EntryKind::Repayment {
                receivable: receivable.parse()?,
                account: account.parse()?,
                amount: amount(n)?,
            },
            Self::Opening { account, balance } => EntryKind::Opening {
                account: account.parse()?,
                balance: Money::from_minor(balance)?,
            },
            Self::Expense {
                account,
                amount: n,
                category,
            } => EntryKind::Expense {
                account: account.parse()?,
                amount: amount(n)?,
                category: Category::from_code(&category)?,
            },
            Self::Income {
                account,
                amount: n,
                category,
                tax,
            } => {
                income_tax = tax.map(StoredIncomeTax::restore).transpose()?;
                EntryKind::Income {
                    account: account.parse()?,
                    amount: amount(n)?,
                    category: Category::from_code(&category)?,
                }
            }
            Self::Transfer {
                from,
                to,
                amount: n,
            } => EntryKind::Transfer {
                from: from.parse()?,
                to: to.parse()?,
                amount: amount(n)?,
            },
            Self::Reversal { original } => {
                let original_id = original.parse()?;
                let original = previous.get(&original_id).ok_or(StorageError::Corrupt)?;
                let entry = JournalEntry::reverse(id, original, note)?;
                if entry.date() != date {
                    return Err(StorageError::Corrupt);
                }
                return Ok(entry);
            }
        };
        let mut entry = JournalEntry::record(id, date, note, kind)?;
        if let Some(tax) = income_tax {
            entry = entry.with_income_tax(tax)?;
        }
        Ok(entry)
    }
}
