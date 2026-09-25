use crate::{components::Icon, i18n::text, state::UiState};
use dioxus::prelude::*;
use ledger_application::{Command, Dashboard};
use ledger_domain::Currency;

#[component]
pub fn SettingsPage(view: Dashboard) -> Element {
    let store = use_context::<UiState>();
    let mut selected = use_signal(|| view.currency);
    let mut tab = use_signal(|| 0usize);
    rsx! {
        section { class: "page-heading settings-page-heading", div { h1 { {text("ตั้งค่า", &[])} } p { class: "muted", {text("จัดสมุดบัญชีให้เป็นของเรา", &[])} } } crate::artwork::Companion {role:"settings"} }
        div { class: "preferences-page",
            div { class: "preferences-tabs", role: "tablist", "aria-label": text("หมวดการตั้งค่า", &[]),
                onkeydown: move |e| {
                    let next = match e.key() {
                        Key::ArrowDown | Key::ArrowRight => (tab() + 1) % 4,
                        Key::ArrowUp | Key::ArrowLeft => (tab() + 3) % 4,
                        Key::Home => 0, Key::End => 3, _ => return,
                    };
                    e.prevent_default(); tab.set(next);
                    let _ = document::eval(&format!("document.getElementById('settings-tab-{next}').focus()"));
                },
                for (index, icon, label) in [(0, "settings", "ทั่วไป"), (1, "sun", "หน้าตา"), (2, "file", "ภาษี"), (3,"download","ข้อมูล")] {
                    button { id: "settings-tab-{index}", r#type: "button", role: "tab", "aria-selected": tab() == index, "aria-controls": "settings-panel-{index}", tabindex: if tab() == index { "0" } else { "-1" }, onclick: move |_| tab.set(index),
                        Icon { name: icon, size: 19 } {text(label, &[])}
                    }
                }
            }
            section { id: "settings-panel-0", class: "preferences-panel", role: "tabpanel", "aria-labelledby": "settings-tab-0", hidden: tab() != 0, tabindex: "0",
                h2 { class: "preferences-section-title", {text("ทั่วไป", &[])} }
                div { class: "settings-group",
                    div { class: "setting-row",
                        div { class: "setting-copy", h3 { {text("ภาษาหน้าจอ", &[])} } p { {text("เปลี่ยนภาษาได้โดยไม่เปลี่ยนยอดหรือสกุลเงิน", &[])} } }
                        div { class: "setting-control", crate::i18n::LanguagePicker {} }
                    }
                    div { class: "setting-row",
                        div { class: "setting-copy", h3 { {text("สกุลเงินสมุดบัญชี", &[])} } p { {text("ภาษาและสกุลเงินแยกกัน หนึ่งสมุดใช้หนึ่งสกุลเงิน ไม่มีการแปลงอัตราแลกเปลี่ยน", &[])} }
                            if view.currency_locked { p { {text("สมุดนี้เริ่มบันทึกแล้ว จึงเปลี่ยนสกุลเงินไม่ได้ สร้างสมุดใหม่เพื่อใช้สกุลเงินอื่น", &[])} } }
                        }
                        div { class: "setting-control currency-control",
                            if view.currency_locked { span { class: "setting-value", "{view.currency.code()} · {view.currency.name()}" } }
                            else {
                                select { "aria-label": text("สกุลเงินสมุดบัญชี", &[]), value: selected().code(), disabled: *store.busy.read(), onchange: move |e| { if let Ok(c) = Currency::from_code(&e.value()) { selected.set(c); } },
                                    for currency in Currency::ALL { option { value: currency.code(), selected: selected() == currency, "{currency.code()} · {currency.name()}" } }
                                }
                                button { class: "soft-button", disabled: *store.busy.read() || selected() == view.currency, onclick: move |_| store.send(Command::SetCurrency(selected())), {text("บันทึก", &[])} }
                            }
                        }
                    }
                }
            }
            section { id: "settings-panel-1", class: "preferences-panel", role: "tabpanel", "aria-labelledby": "settings-tab-1", hidden: tab() != 1, tabindex: "0",
                h2 { class: "preferences-section-title", {text("หน้าตา", &[])} }
                div { class: "settings-group", crate::theme::ThemePicker {} }
            }
            section { id:"settings-panel-3",class:"preferences-panel",role:"tabpanel","aria-labelledby":"settings-tab-3",hidden:tab()!=3,tabindex:"0",crate::data_transfer::DataTransfer {} }
            section { id: "settings-panel-2", class: "preferences-panel", role: "tabpanel", "aria-labelledby": "settings-tab-2", hidden: tab() != 2, tabindex: "0",
                h2 { class: "preferences-section-title", {text("ฟีเจอร์ภาษีไทย", &[])} }
                div { class: "settings-group",
                    div { class: "setting-row",
                        div { class: "setting-copy", h3 { {text("เปิดฟีเจอร์ภาษีไทย", &[])} } p { {crate::i18n::literal("เฉพาะภาษีบุคคลธรรมดาไทยและยอด THB ปิดได้หากไม่ใช้ภาษีไทย สกุลเงินอื่นจะปิดไว้เสมอ")} } }
                        input { class: "settings-switch", r#type: "checkbox", role: "switch", "aria-label": text("เปิดฟีเจอร์ภาษีไทย", &[]), checked: view.thai_tax_enabled, disabled: view.currency != Currency::Thb || *store.busy.read(), onchange: move |e| store.send(Command::SetThaiTaxEnabled(e.checked())) }
                    }
                    div { class: "setting-row",
                        div { class: "setting-copy", h3 { "Open source" } p { {text("ฟีเจอร์นี้รองรับประเทศไทยเท่านั้น หากต้องการภาษีประเทศอื่น สามารถ clone repo แล้วพัฒนาต่อได้ภายใต้ MIT License", &[])} } }
                        div { class: "setting-control settings-links", a { href: "https://github.com/Rayato159/ledgers-bro", target: "_blank", rel: "noopener noreferrer", "GitHub ↗" } a { href: "https://github.com/Rayato159/ledgers-bro/blob/main/LICENSE", target: "_blank", rel: "noopener noreferrer", "MIT License ↗" } }
                    }
                }
            }
        }
    }
}

#[component]
pub fn FirstAccount() -> Element {
    let mut store = use_context::<UiState>();
    let host = use_context::<crate::HostInfo>();
    rsx! {
        section { class: "card first-account",
            img { src: host.art.accounts.clone(), alt: "", class: "first-account-art" }
            span { class: "status-pill", {text("เริ่มจากบัญชีแรก", &[])} }
            h1 { {text("เพิ่มบัญชีก่อนเริ่มจด", &[])} }
            p { {text("เลือกบัญชีที่จะใช้รับหรือจ่ายเงินก่อน", &[])} }
            button { class: "primary", disabled: *store.busy.read(), onclick: move |_| store.account_form.set(true), Icon { name: "plus", size: 19 } {text("เพิ่มบัญชีแรก", &[])} }
            button { class: "text-button", onclick: move |_| store.page.set(crate::state::Page::Settings), {text("ตั้งค่าสกุลเงินก่อนเริ่ม", &[])} }
        }
    }
}
