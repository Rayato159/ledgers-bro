use crate::{AccountId, AccountName, DomainError};

pub const MAX_ACCOUNTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Cash,
    Bank,
    CreditCard,
    Crypto,
    Investment,
}
impl AccountKind {
    pub const ALL: [Self; 5] = [
        Self::Cash,
        Self::Bank,
        Self::CreditCard,
        Self::Crypto,
        Self::Investment,
    ];
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Bank => "bank",
            Self::CreditCard => "credit",
            Self::Crypto => "crypto",
            Self::Investment => "investment",
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cash => "เงินสด",
            Self::Bank => "บัญชีธนาคาร",
            Self::CreditCard => "บัตรเครดิต",
            Self::Crypto => "คริปโต",
            Self::Investment => "พอร์ตหุ้น",
        }
    }
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.code() == code)
            .ok_or(DomainError::AccountUnavailable)
    }
}

/// Identity survives renaming and future archival. A name is not an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    name: AccountName,
    kind: AccountKind,
    archived: bool,
    credit_cycle: Option<crate::CreditCardCycle>,
}
impl Account {
    pub fn new(id: AccountId, name: AccountName, kind: AccountKind) -> Self {
        Self {
            id,
            name,
            kind,
            archived: false,
            credit_cycle: None,
        }
    }
    pub fn restore(id: AccountId, name: AccountName, kind: AccountKind, archived: bool) -> Self {
        Self {
            id,
            name,
            kind,
            archived,
            credit_cycle: None,
        }
    }
    pub const fn id(&self) -> AccountId {
        self.id
    }
    pub fn name(&self) -> &AccountName {
        &self.name
    }
    pub const fn kind(&self) -> AccountKind {
        self.kind
    }
    pub const fn is_archived(&self) -> bool {
        self.archived
    }
    pub const fn credit_cycle(&self) -> Option<crate::CreditCardCycle> {
        self.credit_cycle
    }
    pub fn with_credit_cycle(mut self, cycle: crate::CreditCardCycle) -> Result<Self, DomainError> {
        if self.kind != AccountKind::CreditCard {
            return Err(DomainError::InvalidCreditCycle);
        }
        self.credit_cycle = Some(cycle);
        Ok(self)
    }
    pub fn ensure_can_add(&self, existing: &[Self]) -> Result<(), DomainError> {
        if existing.len() >= MAX_ACCOUNTS {
            return Err(DomainError::AccountLimit);
        }
        if existing
            .iter()
            .any(|a| a.name.key() == self.name.key() || a.id == self.id)
        {
            return Err(DomainError::DuplicateAccountName);
        }
        Ok(())
    }
}
