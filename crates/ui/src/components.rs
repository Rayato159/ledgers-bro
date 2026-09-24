use crate::state::UiState;
use dioxus::prelude::*;
use ledger_domain::*;

#[component]
pub fn Icon(name: &'static str, size: u32) -> Element {
    let path = match name {
        "sun" => {
            "M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM12 2v2M12 20v2M2 12h2M20 12h2M5 5l1 1M18 18l1 1M5 19l1-1M18 6l1-1"
        }
        "moon" => "M20 15A9 9 0 0 1 9 4a9 9 0 1 0 11 11Z",
        "settings" => "M4 7h16M4 17h16M8 4v6M16 14v6",
        "more" => "M4 12h2m5 0h2m5 0h2",
        "chevron" => "m5 9 7 7 7-7",
        "language" => {
            "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18M12 3c-4 5-4 13 0 18 4-5 4-13 0-18Z"
        }
        "home" => "m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1z",
        "wallet" => "M20 8V5a2 2 0 0 0-2-2H5a3 3 0 0 0 0 6h16v11H5a3 3 0 0 1-3-3V6m19 7h-5v4h5",
        "list" => "M8 5h12M8 12h12M8 19h12M3 5h1M3 12h1M3 19h1",
        "notebook" => {
            "M6 3h13a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2ZM8 3v18M2 7h4M2 12h4M2 17h4M12 8h5M12 12h5M12 16h3"
        }
        "chat" => "M21 11a9 9 0 0 1-9 9H4l-2 2V11a9 9 0 0 1 19 0ZM7 10h10M7 14h6",
        "calendar" => "M4 5h16v16H4zM4 10h16M8 2v6M16 2v6M8 14h2M14 14h2M8 18h2",
        "file" => "M14 2H5v20h14V7l-5-5Zm0 0v6h5M8 12h8M8 16h8",
        "plus" => "M12 5v14M5 12h14",
        "edit" => "m16 3 5 5-12 12-6 1 1-6L16 3Zm-3 3 5 5",
        "arrow" => "M5 12h14m-5-5 5 5-5 5",
        "download" => "M12 3v12m-5-5 5 5 5-5M4 17v4h16v-4",
        "device" => "M5 3h14v18H5zM10 18h4",
        "up" => "M7 17 17 7M7 7h10v10",
        "down" => "M7 7 17 17M7 17h10V7",
        "food" => "M5 3v7m4-7v7M3 3v5a4 4 0 0 0 8 0V3M7 12v9M18 3c-3 3-3 8 0 9h2V3h-2Zm2 9v9",
        "camera" => "M8 5 10 2h4l2 3h5v16H3V5Z M16 13a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
        "mic" => "M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0ZM5 10v2a7 7 0 0 0 14 0v-2M12 19v3M8 22h8",
        "stop" => "M6 6h12v12H6Z",
        "check" => "m5 12 4 4L19 6",
        "close" => "m6 6 12 12M6 18 18 6",
        "trash" => "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7",
        "heart" => "M20 5a5 5 0 0 0-8 1 5 5 0 0 0-8-1c-4 5 3 11 8 15 5-4 12-10 8-15Z",
        _ => "M4 5h16v14H4zM8 9h8M8 13h5",
    };
    rsx! { svg { width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.7", stroke_linecap: "round", stroke_linejoin: "round", "aria-hidden": "true", path { d: path } } }
}

pub fn money_label(value: Money) -> String {
    let text = value.to_string();
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, "00"));
    let (sign, unsigned) = whole.strip_prefix('-').map_or(("", whole), |v| ("−", v));
    let mut grouped = String::new();
    for (index, ch) in unsigned.chars().enumerate() {
        if index > 0 && (unsigned.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!("{sign}{grouped}.{fraction}")
}

pub fn account_label(view: &ledger_application::Dashboard, id: AccountId) -> String {
    view.accounts
        .iter()
        .find(|a| a.account.id() == id)
        .map(|a| a.account.name().as_str().to_owned())
        .unwrap_or_else(|| "ไม่พบบัญชี".into())
}

pub fn entry_label(entry: &JournalEntry) -> Option<(&'static str, Money, AccountId)> {
    Some(match *entry.kind() {
        EntryKind::ReceivableOpening { .. } => return None,
        EntryKind::Lending {
            account, amount, ..
        } => ("ให้ยืมเงินต้น", amount.money(), account),
        EntryKind::Repayment {
            account, amount, ..
        } => ("ลูกหนี้ชำระเงินต้น", amount.money(), account),
        EntryKind::Opening { account, balance } => ("ยอดเริ่มต้น", balance, account),
        EntryKind::Income {
            account,
            amount,
            category,
        }
        | EntryKind::Expense {
            account,
            amount,
            category,
        } => (category.label(), amount.money(), account),
        EntryKind::Transfer { from, amount, .. } => ("โอนระหว่างบัญชี", amount.money(), from),
        EntryKind::Reversal { .. } => return None,
    })
}

#[component]
pub fn EmptyState(title: String, body: String) -> Element {
    rsx! { div { class: "empty-state", Icon { name: "wallet", size: 34 } h3 { "{crate::i18n::tr(&title)}" } p { "{crate::i18n::tr(&body)}" } } }
}

#[component]
pub fn PrivacyNote() -> Element {
    rsx! { div { class: "privacy-note", Icon { name: "device", size: 16 } span { {crate::i18n::text("ข้อมูลอยู่ในเครื่องนี้ • ไม่มี Cloud sync", &[])} } } }
}

#[component]
pub fn NewEntryButton() -> Element {
    let mut store = use_context::<UiState>();
    rsx! { button { class: "primary add-entry-button", disabled: *store.busy.read() && store.model_operation.read().is_none(), onclick: move |_| store.page.set(crate::state::Page::Chat), Icon { name: "plus", size: 19 } {crate::i18n::tr("เพิ่มรายการ")} } }
}
