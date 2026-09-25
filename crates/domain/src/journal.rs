use crate::{Account, AccountId, DomainError, EntryDate, EntryId, Money, Note, PositiveMoney};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Rent,
    Food,
    Snacks,
    Luxury,
    Supplies,
    Medical,
    OtherExpense,
    Salary,
    Freelance,
    Interest,
    OtherIncome,
}
impl Category {
    pub const EXPENSE: [Self; 7] = [
        Self::Rent,
        Self::Food,
        Self::Snacks,
        Self::Luxury,
        Self::Supplies,
        Self::Medical,
        Self::OtherExpense,
    ];
    pub const INCOME: [Self; 4] = [
        Self::Salary,
        Self::Freelance,
        Self::Interest,
        Self::OtherIncome,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rent => "ค่าเช่า",
            Self::Food => "อาหาร",
            Self::Snacks => "ของกินเล่น",
            Self::Luxury => "ของฟุ่มเฟือย",
            Self::Supplies => "ของใช้",
            Self::Medical => "รักษาพยาบาล",
            Self::OtherExpense => "อื่นๆ",
            Self::Salary => "เงินเดือน",
            Self::Freelance => "ฟรีแลนซ์",
            Self::Interest => "ดอกเบี้ย",
            Self::OtherIncome => "รายรับอื่นๆ",
        }
    }
    pub const fn code(self) -> &'static str {
        match self {
            Self::Rent => "rent",
            Self::Food => "food",
            Self::Snacks => "snacks",
            Self::Luxury => "luxury",
            Self::Supplies => "supplies",
            Self::Medical => "medical",
            Self::OtherExpense => "other_expense",
            Self::Salary => "salary",
            Self::Freelance => "freelance",
            Self::Interest => "interest",
            Self::OtherIncome => "other_income",
        }
    }
    pub fn from_code(value: &str) -> Result<Self, DomainError> {
        Self::EXPENSE
            .into_iter()
            .chain(Self::INCOME)
            .find(|c| c.code() == value)
            .ok_or(DomainError::InvalidCategory)
    }
    pub fn is_income(self) -> bool {
        Self::INCOME.contains(&self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemBook {
    Receivable,
    Equity,
    Income,
    Expense,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostingTarget {
    Account(AccountId),
    System(SystemBook),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posting {
    target: PostingTarget,
    amount: Money,
}
impl Posting {
    pub const fn target(self) -> PostingTarget {
        self.target
    }
    pub const fn amount(self) -> Money {
        self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    ReceivableOpening {
        receivable: crate::ReceivableId,
        amount: PositiveMoney,
    },
    Lending {
        receivable: crate::ReceivableId,
        account: AccountId,
        amount: PositiveMoney,
    },
    Repayment {
        receivable: crate::ReceivableId,
        account: AccountId,
        amount: PositiveMoney,
    },
    Opening {
        account: AccountId,
        balance: Money,
    },
    Expense {
        account: AccountId,
        amount: PositiveMoney,
        category: Category,
    },
    Income {
        account: AccountId,
        amount: PositiveMoney,
        category: Category,
    },
    Transfer {
        from: AccountId,
        to: AccountId,
        amount: PositiveMoney,
    },
    Reversal {
        original: EntryId,
    },
}

/// Aggregate root. Postings can only be constructed through validated factories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    id: EntryId,
    date: EntryDate,
    note: Note,
    kind: EntryKind,
    postings: Vec<Posting>,
    income_tax: Option<crate::IncomeTax>,
}
impl JournalEntry {
    pub fn record(
        id: EntryId,
        date: EntryDate,
        note: Note,
        kind: EntryKind,
    ) -> Result<Self, DomainError> {
        let (first, second, amount) = match kind {
            EntryKind::ReceivableOpening { amount, .. } => (
                PostingTarget::System(SystemBook::Receivable),
                PostingTarget::System(SystemBook::Equity),
                amount.money(),
            ),
            EntryKind::Lending {
                account, amount, ..
            } => (
                PostingTarget::System(SystemBook::Receivable),
                PostingTarget::Account(account),
                amount.money(),
            ),
            EntryKind::Repayment {
                account, amount, ..
            } => (
                PostingTarget::Account(account),
                PostingTarget::System(SystemBook::Receivable),
                amount.money(),
            ),
            EntryKind::Opening { account, balance } => (
                PostingTarget::Account(account),
                PostingTarget::System(SystemBook::Equity),
                balance,
            ),
            EntryKind::Expense {
                account,
                amount,
                category,
            } => {
                if category.is_income() {
                    return Err(DomainError::InvalidCategory);
                }
                (
                    PostingTarget::Account(account),
                    PostingTarget::System(SystemBook::Expense),
                    amount.money().negated(),
                )
            }
            EntryKind::Income {
                account,
                amount,
                category,
            } => {
                if !category.is_income() {
                    return Err(DomainError::InvalidCategory);
                }
                (
                    PostingTarget::Account(account),
                    PostingTarget::System(SystemBook::Income),
                    amount.money(),
                )
            }
            EntryKind::Transfer { from, to, amount } => {
                if from == to {
                    return Err(DomainError::SameAccountTransfer);
                }
                (
                    PostingTarget::Account(from),
                    PostingTarget::Account(to),
                    amount.money().negated(),
                )
            }
            EntryKind::Reversal { .. } => return Err(DomainError::InvalidReversal),
        };
        Ok(Self {
            id,
            date,
            note,
            kind,
            income_tax: None,
            postings: vec![
                Posting {
                    target: first,
                    amount,
                },
                Posting {
                    target: second,
                    amount: amount.negated(),
                },
            ],
        })
    }
    /// Cancellation corrects the original accounting date, preserving an audit record.
    pub fn reverse(id: EntryId, original: &Self, note: Note) -> Result<Self, DomainError> {
        if matches!(
            original.kind,
            EntryKind::Opening { .. }
                | EntryKind::ReceivableOpening { .. }
                | EntryKind::Reversal { .. }
        ) || id == original.id
        {
            return Err(DomainError::InvalidReversal);
        }
        Ok(Self {
            id,
            date: original.date,
            note,
            income_tax: None,
            kind: EntryKind::Reversal {
                original: original.id,
            },
            postings: original
                .postings
                .iter()
                .map(|p| Posting {
                    target: p.target,
                    amount: p.amount.negated(),
                })
                .collect(),
        })
    }
    pub fn validate_accounts(&self, accounts: &[Account]) -> Result<(), DomainError> {
        let mut sum = 0_i128;
        for posting in &self.postings {
            sum += i128::from(posting.amount.minor());
            if let PostingTarget::Account(id) = posting.target
                && !accounts
                    .iter()
                    .any(|account| account.id() == id && !account.is_archived())
            {
                return Err(DomainError::AccountUnavailable);
            }
        }
        if sum != 0 {
            return Err(DomainError::UnbalancedJournal);
        }
        Ok(())
    }
    pub const fn id(&self) -> EntryId {
        self.id
    }
    pub const fn date(&self) -> EntryDate {
        self.date
    }
    pub fn note(&self) -> &Note {
        &self.note
    }
    pub fn kind(&self) -> &EntryKind {
        &self.kind
    }
    pub const fn income_tax(&self) -> Option<crate::IncomeTax> {
        self.income_tax
    }
    pub fn with_income_tax(mut self, tax: crate::IncomeTax) -> Result<Self, DomainError> {
        let EntryKind::Income { amount, .. } = self.kind else {
            return Err(DomainError::InvalidIncomeTax);
        };
        if tax.net_received()? != amount.money() {
            return Err(DomainError::InvalidIncomeTax);
        }
        self.income_tax = Some(tax);
        Ok(self)
    }
    pub fn postings(&self) -> &[Posting] {
        &self.postings
    }
}
