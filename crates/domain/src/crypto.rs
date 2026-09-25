use crate::{DomainError, Money};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoAsset {
    Bitcoin,
    Solana,
}
impl CryptoAsset {
    pub const ALL: [Self; 2] = [Self::Bitcoin, Self::Solana];
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Bitcoin => "BTC",
            Self::Solana => "SOL",
        }
    }
    pub const fn decimals(self) -> u32 {
        match self {
            Self::Bitcoin => 8,
            Self::Solana => 9,
        }
    }
    pub const fn scale(self) -> u64 {
        10u64.pow(self.decimals())
    }
}

/// Native smallest units: satoshis for BTC and lamports for SOL. Never a float.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoinQuantity {
    asset: CryptoAsset,
    atoms: u64,
}
impl CoinQuantity {
    pub fn new(asset: CryptoAsset, atoms: u64) -> Result<Self, DomainError> {
        if atoms > i64::MAX as u64 {
            return Err(DomainError::InvalidCryptoQuantity);
        }
        Ok(Self { asset, atoms })
    }
    pub fn parse(asset: CryptoAsset, text: &str) -> Result<Self, DomainError> {
        Self::new(
            asset,
            parse_units(text, asset.decimals()).ok_or(DomainError::InvalidCryptoQuantity)?,
        )
    }
    pub const fn atoms(self) -> u64 {
        self.atoms
    }
    pub const fn asset(self) -> CryptoAsset {
        self.asset
    }
}
impl fmt::Display for CoinQuantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scale = self.asset.scale();
        let fraction = format!(
            "{:0width$}",
            self.atoms % scale,
            width = self.asset.decimals() as usize
        );
        let fraction = fraction.trim_end_matches('0');
        write!(f, "{}", self.atoms / scale)?;
        if !fraction.is_empty() {
            write!(f, ".{fraction}")?;
        }
        Ok(())
    }
}

/// A portfolio's holdings are a value of its Account entity, not a cash balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CryptoHoldings {
    bitcoin: CoinQuantity,
    solana: CoinQuantity,
}
impl CryptoHoldings {
    pub fn parse(bitcoin: &str, solana: &str) -> Result<Self, DomainError> {
        Self::from_atoms(
            CoinQuantity::parse(CryptoAsset::Bitcoin, bitcoin)?.atoms(),
            CoinQuantity::parse(CryptoAsset::Solana, solana)?.atoms(),
        )
    }
    pub fn from_atoms(bitcoin: u64, solana: u64) -> Result<Self, DomainError> {
        Ok(Self {
            bitcoin: CoinQuantity::new(CryptoAsset::Bitcoin, bitcoin)?,
            solana: CoinQuantity::new(CryptoAsset::Solana, solana)?,
        })
    }
    pub const fn quantity(self, asset: CryptoAsset) -> CoinQuantity {
        match asset {
            CryptoAsset::Bitcoin => self.bitcoin,
            CryptoAsset::Solana => self.solana,
        }
    }
    pub const fn is_empty(self) -> bool {
        self.bitcoin.atoms == 0 && self.solana.atoms == 0
    }
}

/// THB per coin, at eight decimal places. Round only the final valuation to satang.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThbUnitPrice(u64);
impl ThbUnitPrice {
    pub const SCALE: u64 = 100_000_000;
    pub fn from_units(units: u64) -> Result<Self, DomainError> {
        if units == 0 || units > i64::MAX as u64 {
            return Err(DomainError::InvalidCryptoPrice);
        }
        Ok(Self(units))
    }
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        Self::from_units(parse_units(text, 8).ok_or(DomainError::InvalidCryptoPrice)?)
    }
    pub const fn units(self) -> u64 {
        self.0
    }
    pub fn value(self, quantity: CoinQuantity) -> Result<Money, DomainError> {
        let numerator = u128::from(self.0) * u128::from(quantity.atoms());
        let denominator = u128::from(Self::SCALE / 100) * u128::from(quantity.asset().scale());
        let cents = (numerator + denominator / 2) / denominator;
        Money::from_minor(i64::try_from(cents).map_err(|_| DomainError::MoneyOverflow)?)
    }
    pub fn per_coin(self) -> Result<Money, DomainError> {
        self.value(CoinQuantity::new(
            CryptoAsset::Bitcoin,
            CryptoAsset::Bitcoin.scale(),
        )?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CryptoQuote {
    price: ThbUnitPrice,
    observed_at: i64,
}
impl CryptoQuote {
    pub fn new(price: ThbUnitPrice, observed_at: i64) -> Result<Self, DomainError> {
        if observed_at < 1_230_940_800 {
            return Err(DomainError::InvalidCryptoPrice);
        }
        Ok(Self { price, observed_at })
    }
    pub const fn price(self) -> ThbUnitPrice {
        self.price
    }
    pub const fn observed_at(self) -> i64 {
        self.observed_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CryptoPrices {
    pub bitcoin: CryptoQuote,
    pub solana: CryptoQuote,
}
impl CryptoPrices {
    pub const fn quote(self, asset: CryptoAsset) -> CryptoQuote {
        match asset {
            CryptoAsset::Bitcoin => self.bitcoin,
            CryptoAsset::Solana => self.solana,
        }
    }
    pub fn validate_at(self, now: i64) -> Result<Self, DomainError> {
        if CryptoAsset::ALL
            .iter()
            .any(|a| self.quote(*a).observed_at() > now.saturating_add(300))
        {
            return Err(DomainError::InvalidCryptoPrice);
        }
        Ok(self)
    }
    pub fn oldest_at(self) -> i64 {
        self.bitcoin.observed_at().min(self.solana.observed_at())
    }
    pub fn is_stale(self, now: i64) -> bool {
        now.saturating_sub(self.oldest_at()) > 180 || self.validate_at(now).is_err()
    }
    pub fn value(self, holdings: CryptoHoldings) -> Result<Money, DomainError> {
        self.bitcoin
            .price()
            .value(holdings.bitcoin)?
            .checked_add(self.solana.price().value(holdings.solana)?)
    }
}

fn parse_units(text: &str, decimals: u32) -> Option<u64> {
    let text = text.trim();
    if text.is_empty() || text.len() > 30 {
        return None;
    }
    let mut parts = text.split('.');
    let whole = parts.next()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > decimals as usize
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: u64 = whole.parse().ok()?;
    let fraction_value = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().ok()?
    };
    whole
        .checked_mul(10u64.pow(decimals))?
        .checked_add(fraction_value.checked_mul(10u64.pow(decimals - fraction.len() as u32))?)
}
