use crate::{
    Gateway, HostInfo,
    components::*,
    entry::{ManualEntryPage, QuickEntryPage},
    overview::Overview,
    pages::*,
    state::*,
};
use dioxus::prelude::*;
use ledger_application::Command;

#[component]
pub fn App() -> Element {
    use_context_provider(|| SessionTaskScope(dioxus::dioxus_core::current_scope_id()));
    let mut preferences = use_signal(ledger_application::UserPreferences::default);
    use_context_provider(|| crate::theme::Theme(preferences));
    let mut language = use_signal(crate::i18n::Language::default);
    use_context_provider(|| crate::i18n::Locale(language));
    let gateway = use_context::<Gateway>();
    let preferences_gateway = gateway.clone();
    use_future(move || {
        let gateway = preferences_gateway.clone();
        async move {
            if let Ok(ledger_application::Response::Preferences(saved)) =
                gateway.0.request(Command::LoadPreferences).await
            {
                language.set(if saved.english {
                    crate::i18n::Language::English
                } else {
                    crate::i18n::Language::Thai
                });
                preferences.set(saved);
            }
        }
    });
    use_effect(move || {
        let saved = preferences();
        let theme = if saved.dark { "dark" } else { "light" };
        let lang = if saved.english { "en" } else { "th" };
        let _ = document::eval(&format!(
            "document.documentElement.dataset.theme = '{theme}'; document.documentElement.lang = '{lang}';"
        ));
    });
    let tax_session = use_signal(crate::tax::TaxSession::default);
    use_context_provider(|| tax_session);
    let host = use_context::<HostInfo>();
    let mut store = UiState {
        gateway: use_signal(|| gateway),
        view: use_signal(|| None),
        page: use_signal(|| Page::Overview),
        busy: use_signal(|| false),
        notice: use_signal(|| None),
        account_form: use_signal(|| false),
        export_form: use_signal(|| false),
        account_deletion: use_signal(|| None),
        input: use_signal(|| None),
        prepared: use_signal(|| None),
        guidance: use_signal(String::new),
        composer: use_signal(String::new),
        receipts: use_signal(Vec::new),
        scan_progress: use_signal(String::new),
        scan_cancel: use_signal(|| None),
        model_operation: use_signal(|| None),
        model_choices: use_signal(|| None),
        prompt_drafts: use_signal(|| None),
        prompt_prepared: use_signal(|| None),
        batch: use_signal(|| None),
        batch_prepared: use_signal(|| None),
        recurring_prepared: use_signal(|| None),
        receivable_review: use_signal(|| None),
        repayment_form: use_signal(|| false),
        repayment_selection: use_signal(|| None),
    };
    use_context_provider(|| store);
    let market = crate::crypto::use_crypto_market(store);
    use_effect(move || store.send(Command::Load));
    let page = *store.page.read();
    let view = store.view.read().clone();
    let show_tax = view
        .as_ref()
        .is_some_and(|v| v.thai_tax_enabled && v.currency == ledger_domain::Currency::Thb);
    let notice = store.notice.read().clone();
    rsx! {
        style { {crate::typography::stylesheet()} }
        style { {include_str!("../assets/app.css")} }
        style { {include_str!("../assets/artwork.css")} }
        style { {crate::theme::stylesheet(preferences())} }
        div { class: "app-shell",
            header { class: "topbar",
                button { class: "brand", disabled:*store.busy.read(), onclick: move |_| store.page.set(Page::Overview),
                    span { class: "brand-mark character-brand", img {src:host.art.hero.clone(),alt:""} }
                    span { strong { "ledgers" } span { class: "brand-bro", "bro." } }
                }
                crate::navigation::Navigation { show_tax }
                div { class: "top-actions", NewEntryButton {} }
            }
            main {
                div { class: "ledger-context",
                    crate::profiles::ProfileMenu {}
                    if host.isolated { div { class: "dev-label", "{crate::i18n::tr(host.preview_label)}" } }
                    if let Some(view) = view.as_ref() { span { class: "currency-chip", "{view.currency.code()}" } }
                }
                if let Some((is_error, message)) = notice {
                    div { class: if is_error { "notice error" } else { "notice success" }, role: if is_error { "alert" } else { "status" },
                        span { "{crate::i18n::tr(&message)}" }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.send(Command::Load), {crate::i18n::text("โหลดใหม่", &[])} }
                        button { class: "icon-button", "aria-label": crate::i18n::text("ปิดข้อความ", &[]), onclick: move |_| store.notice.set(None), Icon { name: "close", size: 16 } }
                    }
                }
                if let Some(view) = view {
                    match page {
                        Page::Chat | Page::Manual if !view.accounts.iter().any(|a| a.account.accepts_cash_entries()) => rsx! { crate::currency::FirstAccount {} },
                        Page::Settings => rsx! { crate::currency::SettingsPage { view } },
                        Page::Overview => rsx! { Overview { view } },
                        Page::Accounts => rsx! { AccountsPage { view } },
                        Page::Transactions => rsx! { TransactionsPage { view } },
                        Page::Chat => rsx! { QuickEntryPage { view } },
                        Page::Manual => rsx! { ManualEntryPage { view } },
                        Page::Recurring => rsx! { crate::recurring::RecurringPage { view } },
                        Page::Receivables => rsx! { crate::receivables::ReceivablesPage { view } },
                        Page::Tax if show_tax => rsx! { crate::tax::TaxPage {} },
                        Page::Tax => rsx! { p { {crate::i18n::tr("ฟีเจอร์ภาษีไทยปิดอยู่สำหรับสมุดนี้")} } },
                    }
                } else {
                    div { class: "card startup", h1 { {crate::i18n::text("สมุดบัญชีของเรา", &[])} } p { {crate::i18n::text("กำลังเปิดข้อมูลในเครื่อง…", &[])} } button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::Load), {crate::i18n::text("ลองเปิดอีกครั้ง", &[])} } }
                }
            }
            footer { PrivacyNote {} }
            if *store.account_form.read() { AccountDialog {} }
            if let Some(account) = (market.editing)() { crate::crypto::CryptoHoldingsDialog { account } }
            if *store.export_form.read() { crate::export::ExportDialog {} }
            if *store.repayment_form.read() { if let Some(view) = store.view.read().clone() { crate::receivables::RepaymentDialog { view } } }
            if let Some(deletion) = store.account_deletion.read().clone() {
                crate::account_deletion::DeleteAccountDialog { deletion }
            }
        }
    }
}
