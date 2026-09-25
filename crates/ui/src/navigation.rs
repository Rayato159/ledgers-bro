use crate::{
    components::Icon,
    i18n::tr,
    state::{Page, UiState},
};
use dioxus::prelude::*;

#[component]
pub fn Navigation(show_tax: bool) -> Element {
    let mut store = use_context::<UiState>();
    let mut expanded = use_signal(|| false);
    let page = *store.page.read();
    let more_active = matches!(
        page,
        Page::Recurring | Page::Receivables | Page::Tax | Page::Settings
    );
    rsx! {
        nav { "aria-label": tr("หน้าหลัก"),
            for target in [Page::Overview, Page::Accounts, Page::Transactions] {
                button { class: if page == target { "nav-item active" } else { "nav-item" }, disabled:*store.busy.read(),
                    "aria-current": if page == target { "page" } else { "false" },
                    onclick: move |_| { expanded.set(false); store.page.set(target); },
                    Icon { name: target.icon(), size: 21 } span { "{tr(target.label())}" }
                }
            }
            div { class: if expanded() { "nav-more expanded" } else { "nav-more" },
                onkeydown: move |event| { if event.key() == Key::Escape { expanded.set(false); let _ = document::eval("document.getElementById('more-navigation').focus()"); } },
                button { id: "more-navigation", class: if more_active || expanded() { "nav-item active" } else { "nav-item" },
                    "aria-current": if more_active { "page" } else { "false" },
                    "aria-expanded": expanded(), "aria-controls": "more-navigation-panel",
                    onclick: move |_| expanded.set(!expanded()),
                    Icon { name: "more", size: 21 } span { {tr("เพิ่มเติม")} }
                    span { class: "nav-more-chevron", "aria-hidden": "true", Icon { name: "chevron", size: 13 } }
                }
                if expanded() {
                    button { class: "nav-dismiss", tabindex: "-1", "aria-label": tr("ปิดเมนูเพิ่มเติม"), onclick: move |_| expanded.set(false) }
                    div { id: "more-navigation-panel", class: "nav-more-panel", "aria-label": tr("เมนูเพิ่มเติม"),
                        for target in [Page::Recurring, Page::Receivables, Page::Tax, Page::Settings].into_iter().filter(|p| *p != Page::Tax || show_tax) {
                            button { class: if page == target { "nav-extra active" } else { "nav-extra" }, disabled:*store.busy.read(), "aria-current": if page == target { "page" } else { "false" },
                                onclick: move |_| { expanded.set(false); store.page.set(target); },
                                span { class: "nav-extra-icon", Icon { name: target.icon(), size: 20 } }
                                span { "{tr(target.label())}" }
                                Icon { name: "arrow", size: 16 }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn EntryToolbar(manual: bool) -> Element {
    let mut store = use_context::<UiState>();
    rsx! {
        div { class: "entry-toolbar",
            div { class: "entry-mode", role: "group", "aria-label": tr("วิธีเพิ่มรายการ"),
                button { r#type: "button", "aria-pressed": !manual,
                    disabled: *store.busy.read(), onclick: move |_| store.page.set(Page::Chat),
                    Icon { name: "chat", size: 19 } {tr("พิมพ์ prompt")}
                }
                button { r#type: "button", "aria-pressed": manual,
                    disabled: *store.busy.read(), onclick: move |_| {
                        if store.input.peek().is_none() && store.batch.peek().is_none()
                            && let Some(today) = store.view.peek().as_ref().map(|v| v.today) {
                            store.input.set(Some(ledger_application::EntryInput::empty(today)));
                        }
                        store.page.set(Page::Manual);
                    },
                    Icon { name: "edit", size: 19 } {tr("กรอกเอง")}
                }
            }
            crate::receivables::RepaymentShortcut {}
        }
    }
}
