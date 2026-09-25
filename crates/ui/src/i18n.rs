//! Presentation-only localization. Templates are translated before inserting user data.
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Thai,
    English,
}

#[derive(Clone, Copy)]
pub struct Locale(pub Signal<Language>);

pub fn english() -> bool {
    try_consume_context::<Locale>().is_some_and(|locale| *locale.0.read() == Language::English)
}

/// Fixed references (for example Thai tax requirements) must not adopt ledger units.
pub fn literal(source: &str) -> String {
    translate(source, english()).to_owned()
}

pub fn tr(source: &str) -> String {
    if ENGLISH.iter().any(|(key, _)| *key == source) {
        currency_template(translate(source, english()))
    } else {
        source.to_owned()
    }
}

pub fn text(source: &str, arguments: &[String]) -> String {
    interpolate(&currency_template(translate(source, english())), arguments)
}

pub fn field_list(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| tr(field))
        .collect::<Vec<_>>()
        .join(" · ")
}

pub fn currency() -> ledger_domain::Currency {
    try_consume_context::<crate::state::UiState>()
        .and_then(|store| store.view.read().as_ref().map(|v| v.currency))
        .unwrap_or_default()
}

pub fn currency_prefix() -> String {
    format!("{} ", currency().code())
}

fn currency_template(template: &str) -> String {
    // Only authored UI templates reach this function; inserted user data is untouched.
    let currency = currency();
    let value = template.replace('฿', &currency_prefix());
    if currency == ledger_domain::Currency::Thb {
        value
    } else {
        value
            .replace("เงินบาท", currency.code())
            .replace("บาท", currency.code())
            .replace("THB", currency.code())
            .replace("baht", currency.code())
    }
}

fn translate(source: &str, english: bool) -> &str {
    if english {
        for (key, value) in ENGLISH {
            if *key == source {
                return value;
            }
        }
    }
    source
}

fn interpolate(template: &str, arguments: &[String]) -> String {
    // One pass: braces in account names, notes, and OCR are never interpreted.
    let mut result = String::new();
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        rest = &rest[start..];
        if let Some(end) = rest.find('}')
            && let Ok(index) = rest[1..end].parse::<usize>()
            && let Some(value) = arguments.get(index)
        {
            result.push_str(value);
            rest = &rest[end + 1..];
        } else {
            result.push('{');
            rest = &rest[1..];
        }
    }
    result.push_str(rest);
    result
}

pub fn year(year: i32) -> i32 {
    if english() { year } else { year + 543 }
}

#[component]
pub fn LanguagePicker() -> Element {
    let locale = use_context::<Locale>();
    let theme = use_context::<crate::theme::Theme>();
    let store = use_context::<crate::state::UiState>();
    rsx! {
        div { class: "language-select",
            label { r#for: "display-language", {tr("ภาษา")} }
            select { id: "display-language", value: if theme.0.read().english { "en" } else { "th" }, disabled: *store.busy.read(),
                onchange: move |e| {
                  let next = ledger_application::UserPreferences { english: e.value() == "en", ..*theme.0.peek() };
                  let current = if theme.0.peek().english { "en" } else { "th" };
                  let _ = document::eval(&format!("document.getElementById('display-language').value='{current}'"));
                    crate::theme::persist(store, theme, locale, next);
                },
                option { value: "th", selected: !theme.0.read().english, "ไทย — Thai" }
                option { value: "en", selected: theme.0.read().english, "English" }
            }
        }
    }
}

const ENGLISH: &[(&str, &str)] = include!("../assets/i18n/en.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_never_translated_or_interpreted_as_templates() {
        assert_eq!(
            interpolate("Account {0}: {1}", &["เงินสด {1}".into(), "100".into()]),
            "Account เงินสด {1}: 100"
        );
        assert_eq!(translate("User's own note", true), "User's own note");
        assert_eq!(translate("ภาพรวม", true), "Overview");
        assert_eq!(translate("ภาพรวม", false), "ภาพรวม");
    }

    #[test]
    fn translations_preserve_all_placeholders_and_are_unique() {
        let mut keys = std::collections::HashSet::new();
        for (key, value) in ENGLISH {
            assert!(keys.insert(key), "duplicate translation: {key}");
            for index in 0..20 {
                let marker = format!("{{{index}}}");
                assert_eq!(
                    key.contains(&marker),
                    value.contains(&marker),
                    "placeholder changed: {key}"
                );
            }
        }
    }
}
