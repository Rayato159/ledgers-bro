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
    let gateway = use_context::<Gateway>();
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
        receipt: use_signal(|| None),
        scan_cancel: use_signal(|| None),
        model_operation: use_signal(|| None),
        model_choices: use_signal(|| None),
    };
    use_context_provider(|| store);
    use_effect(move || store.send(Command::Load));
    let page = *store.page.read();
    let navigation_page = if page == Page::Manual {
        Page::Chat
    } else {
        page
    };
    let view = store.view.read().clone();
    let notice = store.notice.read().clone();
    rsx! {
        style { {crate::typography::stylesheet()} }
        style { {include_str!("../assets/app.css")} }
        style { {include_str!("../assets/artwork.css")} }
        div { class: "app-shell",
            header { class: "topbar",
                button { class: "brand", onclick: move |_| store.page.set(Page::Overview),
                    span { class: "brand-mark", Icon { name: "wallet", size: 23 } }
                    span { strong { "ledgers" } span { class: "brand-bro", "bro." } }
                }
                nav { "aria-label": "หน้าหลัก",
                    for target in Page::ALL {
                        button { class: if navigation_page == target { "nav-item active" } else { "nav-item" }, "aria-current": if navigation_page == target { "page" } else { "false" }, onclick: move |_| store.page.set(target), Icon { name: target.icon(), size: 22 } span { "{target.label()}" } }
                    }
                }
                div { class: "top-actions", NewEntryButton {} }
            }
            main {
                if host.isolated { div { class: "dev-label", "{host.preview_label}" } }
                if let Some((is_error, message)) = notice {
                    div { class: if is_error { "notice error" } else { "notice success" }, role: if is_error { "alert" } else { "status" },
                        span { "{message}" }
                        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.send(Command::Load), "โหลดใหม่" }
                        button { class: "icon-button", "aria-label": "ปิดข้อความ", onclick: move |_| store.notice.set(None), Icon { name: "close", size: 16 } }
                    }
                }
                if let Some(view) = view {
                    match page {
                        Page::Overview => rsx! { Overview { view } },
                        Page::Accounts => rsx! { AccountsPage { view } },
                        Page::Transactions => rsx! { TransactionsPage { view } },
                        Page::Chat => rsx! { QuickEntryPage { view } },
                        Page::Manual => rsx! { ManualEntryPage { view } },
                        Page::Tax => rsx! { TaxPage { today: view.today } },
                    }
                } else {
                    div { class: "card startup", h1 { "สมุดบัญชีของเรา" } p { "กำลังเปิดข้อมูลในเครื่อง…" } button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.send(Command::Load), "ลองเปิดอีกครั้ง" } }
                }
            }
            footer { PrivacyNote {} }
            if *store.account_form.read() { AccountDialog {} }
            if *store.export_form.read() { crate::export::ExportDialog {} }
            if let Some(deletion) = store.account_deletion.read().clone() {
                crate::account_deletion::DeleteAccountDialog { deletion }
            }
        }
    }
}
