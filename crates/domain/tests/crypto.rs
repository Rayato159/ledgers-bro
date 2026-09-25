#![allow(clippy::expect_used)]
use ledger_domain::*;

#[test]
fn native_units_round_trip_without_loss_and_reject_ambiguous_or_excess_precision() {
    for (asset, smallest) in [
        (CryptoAsset::Bitcoin, "0.00000001"),
        (CryptoAsset::Solana, "0.000000001"),
    ] {
        let one = CoinQuantity::parse(asset, smallest).expect("smallest unit");
        assert_eq!(one.atoms(), 1);
        assert_eq!(one.to_string(), smallest);
        let max = CoinQuantity::new(asset, i64::MAX as u64).expect("max storage");
        assert_eq!(
            CoinQuantity::parse(asset, &max.to_string()).expect("roundtrip"),
            max
        );
        assert!(CoinQuantity::new(asset, i64::MAX as u64 + 1).is_err());
        for bad in [
            "-1",
            "1,000",
            "1e2",
            "NaN",
            "1.2.3",
            "",
            "99999999999999999999999999999999",
            "0.0000000001",
        ] {
            assert!(CoinQuantity::parse(asset, bad).is_err(), "{bad}");
        }
    }
    assert_eq!(
        CryptoHoldings::parse("0.12000000", "2.123456789")
            .expect("holdings")
            .quantity(CryptoAsset::Bitcoin)
            .to_string(),
        "0.12"
    );
}

#[test]
fn valuation_rounds_only_after_multiplication_and_reports_overflow() {
    let price = ThbUnitPrice::parse("2815675.95613859").expect("price");
    assert_eq!(
        price
            .value(CoinQuantity::parse(CryptoAsset::Bitcoin, "0.12345678").expect("btc"))
            .expect("value")
            .to_string(),
        "347614.29"
    );
    let price = ThbUnitPrice::parse("0.005").expect("half a satang");
    assert_eq!(
        price
            .value(CoinQuantity::parse(CryptoAsset::Solana, "1").expect("coin"))
            .expect("round half up")
            .to_string(),
        "0.01"
    );
    assert_eq!(
        price
            .value(CoinQuantity::parse(CryptoAsset::Solana, "0.999999999").expect("coin"))
            .expect("below half")
            .to_string(),
        "0.00"
    );
    assert!(
        ThbUnitPrice::from_units(i64::MAX as u64)
            .expect("price")
            .value(CoinQuantity::new(CryptoAsset::Bitcoin, i64::MAX as u64).expect("max"))
            .is_err()
    );
    assert!(ThbUnitPrice::parse("0").is_err());
}

#[test]
fn timestamps_distinguish_cached_stale_and_invalid_future_prices() {
    let now = 1_790_330_700;
    let q = CryptoQuote::new(ThbUnitPrice::parse("100").expect("price"), now).expect("quote");
    let prices = CryptoPrices {
        bitcoin: q,
        solana: q,
    };
    assert!(!prices.is_stale(now + 180));
    assert!(prices.is_stale(now + 181));
    assert!(prices.validate_at(now - 301).is_err());
    assert!(prices.validate_at(now - 300).is_ok());
    assert!(CryptoQuote::new(q.price(), 0).is_err());
}
