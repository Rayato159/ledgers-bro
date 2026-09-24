//! One currency per ledger. Supported currencies all use two decimal minor units.
use crate::DomainError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Currency {
    #[default]
    Thb,
    Usd,
    Eur,
    Gbp,
    Aud,
    Cad,
    Sgd,
    Cny,
}
impl Currency {
    pub const ALL: [Self; 8] = [
        Self::Thb,
        Self::Usd,
        Self::Eur,
        Self::Gbp,
        Self::Aud,
        Self::Cad,
        Self::Sgd,
        Self::Cny,
    ];
    pub const fn code(self) -> &'static str {
        match self {
            Self::Thb => "THB",
            Self::Usd => "USD",
            Self::Eur => "EUR",
            Self::Gbp => "GBP",
            Self::Aud => "AUD",
            Self::Cad => "CAD",
            Self::Sgd => "SGD",
            Self::Cny => "CNY",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::Thb => "Thai baht",
            Self::Usd => "US dollar",
            Self::Eur => "Euro",
            Self::Gbp => "British pound",
            Self::Aud => "Australian dollar",
            Self::Cad => "Canadian dollar",
            Self::Sgd => "Singapore dollar",
            Self::Cny => "Chinese yuan",
        }
    }
    pub fn from_code(code: &str) -> Result<Self, DomainError> {
        Self::ALL
            .into_iter()
            .find(|c| c.code() == code)
            .ok_or(DomainError::InvalidCurrency)
    }
}
