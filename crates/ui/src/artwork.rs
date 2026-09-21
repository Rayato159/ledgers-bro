//! Editable category illustrations use the shared pastel palette.
use dioxus::prelude::*;
use ledger_domain::{AccountKind, Category, EntryKind};

pub const fn category_art(category: Category) -> &'static str {
    match category {
        Category::Rent => "rent",
        Category::Food => "food",
        Category::Snacks => "snacks",
        Category::Luxury => "luxury",
        Category::Supplies => "supplies",
        Category::Medical => "medical",
        Category::Salary => "salary",
        Category::Freelance => "freelance",
        Category::Interest => "interest",
        Category::OtherExpense | Category::OtherIncome => "other",
    }
}
pub const fn account_art(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Cash => "cash",
        AccountKind::Bank => "bank",
        AccountKind::CreditCard => "credit",
        AccountKind::Crypto => "crypto",
        AccountKind::Investment => "investment",
    }
}
pub const fn entry_art(kind: &EntryKind) -> &'static str {
    match kind {
        EntryKind::Expense { category, .. } | EntryKind::Income { category, .. } => {
            category_art(*category)
        }
        EntryKind::Transfer { .. } => "transfer",
        _ => "other",
    }
}

#[component]
pub fn ArtIcon(name: &'static str, size: u32) -> Element {
    let drawing = match name {
        "food" => rsx! {
            path { d: "M10 30Q13 50 28 50Q43 50 46 30Z", fill: "var(--art-primary)" }
            ellipse { cx: "28", cy: "30", rx: "18", ry: "7", fill: "var(--art-paper)" }
            path { d: "M16 29q4-7 8-1q4-7 8 0q4-5 8 1M18 15l20 14M25 12l18 14M21 41q6 3 13-1", fill: "none" }
        },
        "snacks" => rsx! {
            path { d: "M14 23h26l-3 27H17Z", fill: "var(--art-primary)" }
            path { d: "M12 20h30v5H12Z", fill: "var(--art-paper)" }
            path { d: "m30 19 3-12 10-3M15 32h24v10H16Z", fill: "var(--art-primary)" }
            circle { cx: "24", cy: "45", r: "1.5", fill: "var(--art-outline)" }
            circle { cx: "30", cy: "46", r: "1.5", fill: "var(--art-outline)" }
            circle { cx: "27", cy: "43", r: "1.5", fill: "var(--art-outline)" }
        },
        "rent" => rsx! {
            path { d: "M12 27v23h32V26L28 14Z", fill: "var(--art-paper)" }
            path { d: "m7 27 21-18 21 18-5 5L28 19 12 32Z", fill: "var(--art-secondary)" }
            path { d: "M23 35h10v15H23Z", fill: "var(--art-primary)" }
            path { d: "M14 34h5v7h-5Zm23 0h5v7h-5Z", fill: "var(--art-paper)" }
        },
        "luxury" => rsx! {
            path { d: "M10 23h36l-3 27H13Z", fill: "var(--art-primary)" }
            path { d: "M20 25V16a8 8 0 0 1 16 0v9M15 30l2 15", fill: "none" }
            path { d: "m35 30 2 5 5 2-5 2-2 5-2-5-5-2 5-2Z", fill: "var(--art-paper)" }
        },
        "supplies" => rsx! {
            path { d: "M12 25h14v25H12Z", fill: "var(--art-primary)" }
            path { d: "M16 25V15h13M23 15v4", fill: "none" }
            path { d: "M12 34h14v9H12Z", fill: "var(--art-paper)" }
            path { d: "M32 19h10l3 31H29Z", fill: "var(--art-primary)" }
            path { d: "M33 13h8v6h-8ZM32 33h10", fill: "var(--art-paper)" }
        },
        "medical" => rsx! {
            rect { x: "9", y: "21", width: "38", height: "29", rx: "6", fill: "var(--art-paper)" }
            path { d: "M20 21v-8h16v8M10 28h36", fill: "none" }
            path { d: "M25 31h6v5h5v6h-5v5h-6v-5h-5v-6h5Z", fill: "var(--art-secondary)" }
        },
        "salary" | "freelance" => rsx! {
            rect { x: "8", y: "21", width: "40", height: "29", rx: "5", fill: "var(--art-primary)" }
            path { d: "M20 20v-8h16v8M9 30q20 12 38 0", fill: "none" }
            path { d: "M24 31h8v10h-8Z", fill: "var(--art-paper)" }
        },
        "interest" | "investment" => rsx! {
            path { d: "M13 39h30l-4 11H17Z", fill: "var(--art-primary)" }
            path { d: "M28 40V15", fill: "none" }
            path { d: "M28 29Q10 31 11 15Q27 12 28 29Z", fill: "var(--art-primary)" }
            path { d: "M28 24Q44 24 46 8Q30 6 28 24Z", fill: "var(--art-secondary)" }
        },
        "cash" => rsx! {
            path { d: "M10 15h33v26H10Z", fill: "var(--art-secondary)", transform: "rotate(-9 27 28)" }
            rect { x: "9", y: "25", width: "40", height: "25", rx: "5", fill: "var(--art-secondary)" }
            path { d: "M38 32h12v12H38Z", fill: "var(--art-primary)" }
            circle { cx: "43", cy: "38", r: "1", fill: "var(--art-outline)" }
        },
        "bank" => rsx! {
            path { d: "m7 22 21-13 21 13ZM10 46h36v5H10Z", fill: "var(--art-primary)" }
            path { d: "M13 25h6v21h-6Zm12 0h6v21h-6Zm12 0h6v21h-6Z", fill: "var(--art-paper)" }
        },
        "credit" => rsx! {
            rect { x: "7", y: "15", width: "43", height: "33", rx: "5", fill: "var(--art-primary)" }
            path { d: "M8 23h41v8H8Z", fill: "var(--art-outline)" }
            path { d: "M14 38h9v5h-9ZM29 41h5m5 0h4", fill: "var(--art-primary)" }
        },
        "crypto" => rsx! {
            circle { cx: "28", cy: "30", r: "21", fill: "var(--art-primary)" }
            circle { cx: "28", cy: "30", r: "16", fill: "var(--art-primary)" }
            path { d: "M22 18v24m5-26v4m0 19v5M19 20h11q11 4 0 10H22h9q11 6 0 10H19", fill: "none" }
        },
        "receipt" => rsx! {
            path { d: "M13 7h30v44l-5-3-5 3-5-3-5 3-5-3-5 3Z", fill: "var(--art-paper)" }
            path { d: "M20 16h16M20 23h11M20 30h16M20 38h8", fill: "none" }
            path { d: "m32 38 4 4 10-12", stroke: "var(--art-outline)", stroke_width: "3", fill: "none" }
        },
        "transfer" => rsx! {
            path { d: "M9 20h36L35 10M45 20 35 30", stroke: "var(--art-secondary)", stroke_width: "4", fill: "none" }
            path { d: "M47 40H11l10 10M11 40l10-10", stroke: "var(--art-secondary)", stroke_width: "4", fill: "none" }
        },
        _ => rsx! {
            path { d: "M10 25h36v25H10ZM7 20h42v9H7Z", fill: "var(--art-primary)" }
            path { d: "M25 20h7v30h-7Z", fill: "var(--art-primary)" }
            path { d: "M28 20C6 20 16 0 28 20c12-20 24 0 0 0Z", fill: "var(--art-primary)" }
        },
    };
    rsx! { svg { class: "art-icon art-icon-{name}", width: "{size}", height: "{size}", view_box: "0 0 56 56", "aria-hidden": "true", fill: "none", stroke: "var(--art-outline)", stroke_width: "2.1", stroke_linecap: "round", stroke_linejoin: "round",
        ellipse { cx: "28", cy: "50", rx: "22", ry: "3", fill: "#00000020", stroke: "none" }
        {drawing}
    } }
}
