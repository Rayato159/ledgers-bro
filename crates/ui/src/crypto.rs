use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::{Command, Response};
use ledger_domain::*;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(crate) struct CryptoMarket {
    pub prices: Signal<Option<CryptoPrices>>,
    pub now: Signal<i64>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    attempted: Signal<Option<Instant>>,
    failures: Signal<u32>,
    pub editing: Signal<Option<Account>>,
}

pub(crate) fn use_crypto_market(store: UiState) -> CryptoMarket {
    let mut market = CryptoMarket {
        prices: use_signal(|| None),
        now: use_signal(|| chrono::Utc::now().timestamp()),
        loading: use_signal(|| false),
        error: use_signal(|| None),
        attempted: use_signal(|| None),
        failures: use_signal(|| 0),
        editing: use_signal(|| None),
    };
    use_context_provider(|| market);
    use_future(move || async move {
        let gateway = store.gateway.peek().clone();
        if let Ok(Response::CryptoPrices(prices)) =
            gateway.0.request(Command::LoadCryptoPrices).await
        {
            // A user refresh may complete before the initial cache read.
            if market.prices.peek().is_none() {
                market
                    .prices
                    .set(prices.filter(|p| p.validate_at(chrono::Utc::now().timestamp()).is_ok()));
            }
        }
        loop {
            market.now.set(chrono::Utc::now().timestamp());
            let has_holdings = store.view.peek().as_ref().is_some_and(|v| {
                v.accounts
                    .iter()
                    .any(|a| a.account.crypto_holdings().is_some_and(|h| !h.is_empty()))
            });
            let retry_after = Duration::from_secs(60 * 2u64.pow((*market.failures.peek()).min(3)));
            let ready = market
                .attempted
                .peek()
                .is_none_or(|at| at.elapsed() >= retry_after);
            if has_holdings && ready && !*market.loading.peek() {
                market.refresh(store);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    market
}

impl CryptoMarket {
    fn can_refresh(self) -> bool {
        !*self.loading.read()
            && self
                .attempted
                .read()
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
    }
    fn refresh(mut self, store: UiState) {
        if !self.can_refresh() {
            return;
        }
        self.loading.set(true);
        self.attempted.set(Some(Instant::now()));
        let gateway = store.gateway.peek().clone();
        // Owned by the root scope; navigating away must not strand a busy signal.
        crate::state::spawn_session(async move {
            match gateway.0.crypto_prices().await {
                Ok(prices) => {
                    match gateway.0.request(Command::SaveCryptoPrices(prices)).await {
                        Ok(Response::CryptoPrices(Some(saved))) => {
                            self.prices.set(Some(saved));
                            self.error.set(None);
                        }
                        _ => {
                            self.prices.set(Some(prices));
                            self.error
                                .set(Some("อัปเดตราคาแล้ว แต่บันทึกไว้ใช้ครั้งหน้าไม่สำเร็จ".into()));
                        }
                    }
                    self.failures.set(0);
                }
                Err(error) => {
                    self.error.set(Some(error.to_string()));
                    let failures = (*self.failures.peek()).saturating_add(1);
                    self.failures.set(failures);
                }
            }
            self.now.set(chrono::Utc::now().timestamp());
            self.loading.set(false);
        });
    }
}

#[component]
pub(crate) fn CryptoMarketStatus() -> Element {
    let market = use_context::<CryptoMarket>();
    let store = use_context::<UiState>();
    let now = (market.now)();
    let prices = (market.prices)();
    let error = (market.error)();
    let stale = prices.is_some_and(|p| p.is_stale(now));
    let timestamp = prices
        .and_then(|p| chrono::DateTime::from_timestamp(p.oldest_at(), 0))
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format("%d/%m/%Y %H:%M:%S")
                .to_string()
        });
    rsx! {
        section { class: "card crypto-market", "aria-label": crate::i18n::tr("ราคาตลาดคริปโต"),
            div { class: "crypto-market-heading",
                div { h2 { {crate::i18n::tr("ราคาตลาดคริปโต")} }
                    p { class: "muted small", {crate::i18n::tr("มูลค่าประเมิน THB · อัปเดตอัตโนมัติทุกประมาณ 60 วินาที")}
                        " · " a { href: "https://www.coingecko.com/en/api", target: "_blank", rel: "noopener noreferrer", "CoinGecko" }
                    }
                }
                button { class: "soft-button", disabled: !market.can_refresh(), onclick: move |_| market.refresh(store),
                    Icon { name: "refresh", size: 18 }
                    if (market.loading)() { {crate::i18n::tr("กำลังอัปเดตราคา…")} } else { {crate::i18n::tr("อัปเดตราคา")} }
                }
            }
            if let Some(prices) = prices {
                div { class: "crypto-price-strip", for asset in CryptoAsset::ALL {
                    span { strong { "{asset.symbol()}" } {format!(" ≈ THB {}", prices.quote(asset).price().per_coin().map(money_label).unwrap_or_else(|_| "—".into()))} }
                } }
            }
            p { class: if stale || error.is_some() { "field-hint crypto-stale" } else { "field-hint" },
                if let Some(timestamp) = timestamp { {crate::i18n::text("ราคาจากผู้ให้บริการ ณ {0}", &[timestamp])} }
                else { {crate::i18n::tr("ยังไม่มีราคา พอร์ตที่มีเหรียญจะยังไม่รวมในสินทรัพย์") } }
                if stale { " · " {crate::i18n::tr("ราคาเก่า กำลังใช้ข้อมูลล่าสุดที่บันทึกไว้")} }
            }
            if let Some(error) = &error { p { class: "field-hint crypto-stale", role: "status", "{crate::i18n::tr(error)}" } }
        }
    }
}

#[component]
pub(crate) fn CryptoAmounts(mut bitcoin: Signal<String>, mut solana: Signal<String>) -> Element {
    let store = use_context::<UiState>();
    rsx! {
        div { class: "crypto-amount-fields",
            div { label { r#for: "crypto-btc", "Bitcoin (BTC)" }
                input { id: "crypto-btc", inputmode: "decimal", required: true, maxlength: 30, value: "{bitcoin}", disabled: (store.busy)(), oninput: move |e| bitcoin.set(e.value()) }
                small { class: "field-hint", {crate::i18n::tr("จำนวนเหรียญ · ทศนิยมสูงสุด 8 ตำแหน่ง")} }
            }
            div { label { r#for: "crypto-sol", "Solana (SOL)" }
                input { id: "crypto-sol", inputmode: "decimal", required: true, maxlength: 30, value: "{solana}", disabled: (store.busy)(), oninput: move |e| solana.set(e.value()) }
                small { class: "field-hint", {crate::i18n::tr("จำนวนเหรียญ · ทศนิยมสูงสุด 9 ตำแหน่ง")} }
            }
        }
        p { class: "field-hint", {crate::i18n::tr("ระบุจำนวนเหรียญที่ถือจริง ใส่ 0 สำหรับเหรียญที่ไม่มี ราคาตลาดไม่ถูกนับเป็นรายรับหรือภาษีอัตโนมัติ")} }
    }
}

#[component]
pub(crate) fn CryptoAccountCard(account: Account, book_balance: Money) -> Element {
    let store = use_context::<UiState>();
    let mut market = use_context::<CryptoMarket>();
    let holdings = account.crypto_holdings();
    let prices = (market.prices)().filter(|p| p.validate_at((market.now)()).is_ok());
    let value = holdings.and_then(|h| {
        if h.is_empty() {
            Some(Ok(Money::ZERO))
        } else {
            prices.map(|p| p.value(h))
        }
    });
    let invalid_value = value.as_ref().is_some_and(|v| v.is_err());
    let expected = account.clone();
    let id = account.id();
    rsx! {
        article { class: "card account-card crypto-account",
            span { class: "account-icon", crate::artwork::ArtIcon { name: "crypto", size: 48 } }
            small { {crate::i18n::tr("คริปโต")} } h2 { "{account.name().as_str()}" }
            if let Some(holdings) = holdings {
                div { class: "crypto-account-total",
                    strong { "THB " {value.and_then(Result::ok).map(money_label).unwrap_or_else(|| "—".into())} }
                    small { {crate::i18n::tr("มูลค่าประเมินรวม")} }
                    if invalid_value { small { class: "form-error", {crate::i18n::tr("มูลค่าเกินขอบเขตที่คำนวณได้")} } }
                }
                div { class: "crypto-holdings", for asset in CryptoAsset::ALL {
                    div { class: "crypto-holding", span { class: "coin-symbol", "{asset.symbol()}" }
                        div { strong { "{holdings.quantity(asset)}" }
                            small { class: "muted", if let Some(prices) = prices { {format!("≈ THB {}", prices.quote(asset).price().value(holdings.quantity(asset)).map(money_label).unwrap_or_else(|_| "—".into()))} } else { "— THB" } }
                        }
                    }
                } }
            } else {
                strong { "{crate::i18n::currency_prefix()}{money_label(book_balance)}" }
                p { class: "muted small", {crate::i18n::tr("ยอดเดิมที่บันทึกด้วยมือ · ยังไม่ได้ระบุจำนวนเหรียญ")} }
            }
            div { class: "crypto-account-actions",
                button { class: "soft-button", disabled: (store.busy)(), onclick: move |_| market.editing.set(Some(expected.clone())),
                    Icon { name: "edit", size: 18 }
                    if holdings.is_some() { {crate::i18n::tr("แก้จำนวนเหรียญ")} } else { {crate::i18n::tr("ระบุจำนวนเหรียญ")} }
                }
                button { class: "danger-button", disabled: (store.busy)(), "aria-label": crate::i18n::text("ลบบัญชี {0}", &[account.name().as_str().into()]),
                    onclick: move |_| store.send(Command::PreviewDeleteAccount(id)), Icon { name: "trash", size: 20 } {crate::i18n::tr("ลบบัญชี")}
                }
            }
        }
    }
}

#[component]
pub(crate) fn CryptoHoldingsDialog(account: Account) -> Element {
    let mut store = use_context::<UiState>();
    let mut market = use_context::<CryptoMarket>();
    let bitcoin = use_signal(|| {
        account
            .crypto_holdings()
            .map(|h| h.quantity(CryptoAsset::Bitcoin).to_string())
            .unwrap_or_else(|| "0".into())
    });
    let solana = use_signal(|| {
        account
            .crypto_holdings()
            .map(|h| h.quantity(CryptoAsset::Solana).to_string())
            .unwrap_or_else(|| "0".into())
    });
    let mut error = use_signal(|| None::<String>);
    let expected = account.clone();
    rsx! {
        dialog { id: "crypto-dialog", class: "account-dialog", "aria-labelledby": "crypto-title",
            onmounted: move |_| { let _ = document::eval("document.getElementById('crypto-dialog').showModal()"); },
            oncancel: move |e| { e.prevent_default(); if !(store.busy)() { market.editing.set(None); } },
            form { onsubmit: move |e| {
                e.prevent_default();
                if (store.busy)() { return; }
                let holdings = match CryptoHoldings::parse(&bitcoin(), &solana()) {
                    Ok(holdings) => holdings,
                    Err(e) => { error.set(Some(e.to_string())); return; }
                };
                let expected = expected.clone();
                let gateway = store.gateway.peek().clone();
                store.busy.set(true);
                error.set(None);
                spawn(async move {
                    match gateway.0.request(Command::SetCryptoHoldings { expected, holdings }).await {
                        Ok(_) => {
                            match gateway.0.request(Command::Load).await {
                                Ok(Response::Dashboard(view)) => {
                                    store.view.set(Some(view));
                                    store.notice.set(Some((false, "บันทึกจำนวนเหรียญแล้ว".into())));
                                }
                                _ => store.notice.set(Some((true, "บันทึกจำนวนเหรียญแล้ว แต่โหลดภาพรวมไม่สำเร็จ กรุณาโหลดใหม่".into()))),
                            }
                            market.editing.set(None);
                        }
                        Err(e) => error.set(Some(e.to_string())),
                    }
                    store.busy.set(false);
                });
            },
                div { class: "section-heading", h2 { id: "crypto-title", "{account.name().as_str()}" }
                    button { r#type: "button", class: "icon-button", disabled: (store.busy)(), "aria-label": crate::i18n::tr("ปิดหน้าต่าง"), onclick: move |_| market.editing.set(None), Icon { name: "close", size: 20 } }
                }
                CryptoAmounts { bitcoin, solana }
                if account.crypto_holdings().is_none() {
                    p { class: "inline-note", {crate::i18n::tr("เมื่อตั้งจำนวนเหรียญ ภาพรวมจะใช้มูลค่าตลาดแทนยอดเดิม ประวัติรายการยังอยู่ และพอร์ตนี้จะใช้เป็นบัญชีรับจ่ายเงินไม่ได้")} }
                }
                if let Some(error) = error() { p { class: "form-error", role: "alert", "{crate::i18n::tr(&error)}" } }
                button { class: "primary full-width", r#type: "submit", disabled: (store.busy)(), {crate::i18n::tr("บันทึกจำนวนเหรียญ")} }
            }
        }
    }
}
