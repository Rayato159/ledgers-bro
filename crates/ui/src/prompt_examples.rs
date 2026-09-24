use crate::{artwork::ArtIcon, components::Icon};
use dioxus::prelude::*;
use ledger_application::PromptKind;

#[component]
pub(crate) fn PromptExample(
    title: String,
    sample: String,
    icon: &'static str,
    #[props(default = "lilac")] tone: &'static str,
    #[props(default)] compact: bool,
    #[props(default)] selected: bool,
    disabled: bool,
    onselect: EventHandler<String>,
) -> Element {
    let sample = currency_example(&sample);
    rsx! {
        button {
            class: if compact { "prompt-example compact" } else { "prompt-example" },
            r#type: "button", "data-tone": tone, "aria-pressed": selected, disabled,
            onclick: move |_| {
                onselect.call(sample.clone());
                let _ = document::eval("const input = document.getElementById('quick-text'); if (input) { input.focus({preventScroll: true}); input.scrollIntoView({block: 'center', behavior: 'instant'}); }");
            },
            span { class: "prompt-example-heading",
                span { class: "prompt-example-icon", ArtIcon { name: icon, size: if compact { 32 } else { 40 } } }
                strong { "{crate::i18n::tr(&title)}" }
                if selected { span { class: "prompt-example-selected", Icon { name: "check", size: 16 } } }
            }
            span { class: "prompt-example-text", lang: "th", "{sample}" }
            span { class: "prompt-example-action", if selected { {crate::i18n::text("เลือกแล้ว · แก้ข้อความได้", &[])} } else { {crate::i18n::text("ใช้ตัวอย่างนี้", &[])} } Icon { name: "arrow", size: 15 } }
        }
    }
}

pub(crate) fn example_art(kind: PromptKind) -> (&'static str, &'static str) {
    match kind {
        PromptKind::Expense => ("food", "peach"),
        PromptKind::Income => ("salary", "mint"),
        PromptKind::Transfer => ("transfer", "blue"),
        PromptKind::Account => ("cash", "mint"),
        PromptKind::Recurring
        | PromptKind::PayRecurring
        | PromptKind::LinkRecurring
        | PromptKind::CountRecurring
        | PromptKind::StopRecurring => ("calendar", "lilac"),
        PromptKind::Receivable | PromptKind::Lending => ("bank", "blue"),
        PromptKind::Repayment => ("interest", "mint"),
        PromptKind::Reverse | PromptKind::DeleteAccount => ("receipt", "peach"),
    }
}

pub(crate) fn currency_example(sample: &str) -> String {
    if crate::i18n::currency() == ledger_domain::Currency::Thb {
        sample.to_owned()
    } else {
        sample.replace("บาท", crate::i18n::currency().code())
    }
}
