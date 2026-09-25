use ledger_application::{AppError, CryptoPriceSource};
use ledger_domain::{CryptoPrices, CryptoQuote, ThbUnitPrice};
use serde::Deserialize;
use std::{io::Read, time::Duration};

const PRICE_URL: &str = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,solana&vs_currencies=thb&include_last_updated_at=true&precision=8";
const MAX_RESPONSE: u64 = 16 * 1024;

pub struct CoinGeckoPrices;
impl CryptoPriceSource for CoinGeckoPrices {
    fn fetch(&self, now: i64) -> Result<CryptoPrices, AppError> {
        let unavailable = |_| AppError::Input("อัปเดตราคาตลาดไม่ได้ กำลังแสดงราคาที่บันทึกไว้ล่าสุด".into());
        let client = reqwest::blocking::Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(8))
            .user_agent(concat!("LedgersBro/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(unavailable)?;
        // Only these public coin IDs and THB are sent; no holdings, account names,
        // wallet addresses, or ledger data leave the device.
        let response = client.get(PRICE_URL).send().map_err(unavailable)?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AppError::Input(
                "ผู้ให้บริการจำกัดการเรียกราคา กรุณารอรอบอัปเดตถัดไป".into(),
            ));
        }
        let response = response.error_for_status().map_err(unavailable)?;
        let mut bytes = Vec::new();
        response
            .take(MAX_RESPONSE + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::Input("อ่านราคาตลาดไม่สำเร็จ".into()))?;
        parse_response(&bytes, now)
    }
}

#[derive(Deserialize)]
struct PriceRow {
    thb: serde_json::Number,
    last_updated_at: i64,
}
#[derive(Deserialize)]
struct PriceResponse {
    bitcoin: PriceRow,
    solana: PriceRow,
}
fn parse_response(bytes: &[u8], now: i64) -> Result<CryptoPrices, AppError> {
    if bytes.len() as u64 > MAX_RESPONSE {
        return Err(ledger_domain::DomainError::InvalidCryptoPrice.into());
    }
    let decoded: PriceResponse = serde_json::from_slice(bytes)
        .map_err(|_| ledger_domain::DomainError::InvalidCryptoPrice)?;
    let quote = |row: PriceRow| {
        CryptoQuote::new(
            ThbUnitPrice::parse(&row.thb.to_string())?,
            row.last_updated_at,
        )
    };
    Ok(CryptoPrices {
        bitcoin: quote(decoded.bitcoin)?,
        solana: quote(decoded.solana)?,
    }
    .validate_at(now)?)
}

/// Bounded blocking network I/O on a separate thread, never on the ledger worker
/// or UI executor. Shared by Windows and Android adapters.
pub async fn fetch_crypto_prices() -> Result<CryptoPrices, AppError> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    std::thread::Builder::new()
        .name("crypto-prices".into())
        .spawn(move || {
            let result = CoinGeckoPrices.fetch(chrono::Utc::now().timestamp());
            let _ = sender.send(result);
        })
        .map_err(|_| AppError::WorkerStopped)?;
    receiver.await.map_err(|_| AppError::WorkerStopped)?
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use ledger_domain::{CoinQuantity, CryptoAsset};
    #[test]
    fn exact_decimal_quotes_and_untrusted_provider_data() {
        let prices = parse_response(br#"{"bitcoin":{"thb":2815675.95613859,"last_updated_at":1790330680},"solana":{"thb":3937.37225198,"last_updated_at":1790330670}}"#,1790330700).expect("quotes");
        assert_eq!(prices.bitcoin.price().units(), 281567595613859);
        assert_eq!(
            prices
                .bitcoin
                .price()
                .value(CoinQuantity::parse(CryptoAsset::Bitcoin, "0.12345678").expect("quantity"))
                .expect("value")
                .to_string(),
            "347614.29"
        );
        for bad in [br#"{}"#.as_slice(), br#"{"bitcoin":{"thb":0,"last_updated_at":1790330680},"solana":{"thb":1,"last_updated_at":1790330680}}"#,br#"{"bitcoin":{"thb":100,"last_updated_at":1999999999},"solana":{"thb":1,"last_updated_at":1790330680}}"#,br#"{"bitcoin":{"thb":null,"last_updated_at":1790330680},"solana":{"thb":1,"last_updated_at":1790330680}}"#] { assert!(parse_response(bad,1790330700).is_err()); }
        assert!(parse_response(&vec![b' '; MAX_RESPONSE as usize + 1], 1790330700).is_err());
    }
}
