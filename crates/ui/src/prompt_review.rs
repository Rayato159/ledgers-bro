use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use ledger_domain::*;

fn edit(mut store: UiState, index: usize, key: &str, value: String) {
    store.prompt_prepared.set(None);
    store.notice.set(None);
    if let Some((_, drafts)) = store.prompt_drafts.write().as_mut()
        && let Some(draft) = drafts.get_mut(index)
    {
        draft.set(key, value);
    }
}
fn options(kind: PromptFieldType, view: &Dashboard) -> Vec<(String, String)> {
    match kind {
        PromptFieldType::Account => view
            .accounts
            .iter()
            .filter(|a| a.account.accepts_cash_entries())
            .map(|a| (a.account.id().to_string(), a.account.name().as_str().into()))
            .collect(),
        PromptFieldType::AccountKind => AccountKind::ALL
            .into_iter()
            .map(|k| (k.code().into(), crate::i18n::tr(k.label())))
            .collect(),
        PromptFieldType::ExpenseCategory => Category::EXPENSE
            .into_iter()
            .map(|c| (c.code().into(), crate::i18n::tr(c.label())))
            .collect(),
        PromptFieldType::IncomeCategory => Category::INCOME
            .into_iter()
            .map(|c| (c.code().into(), crate::i18n::tr(c.label())))
            .collect(),
        PromptFieldType::Recurring => view
            .recurring
            .iter()
            .map(|p| {
                (
                    p.id().to_string(),
                    format!("{} · เริ่ม {}", p.name().as_str(), p.due().start()),
                )
            })
            .collect(),
        PromptFieldType::Receivable => view
            .receivables
            .iter()
            .map(|p| {
                (
                    p.id().to_string(),
                    format!("{} · {}", p.debtor().as_str(), p.description().as_str()),
                )
            })
            .collect(),
        PromptFieldType::Entry => view
            .entries
            .iter()
            .filter(|e| {
                !view.reversed.contains(&e.id())
                    && !matches!(
                        e.kind(),
                        EntryKind::Opening { .. }
                            | EntryKind::ReceivableOpening { .. }
                            | EntryKind::Reversal { .. }
                    )
            })
            .map(|e| {
                (
                    e.id().to_string(),
                    format!(
                        "{} · {} · {}",
                        e.date(),
                        entry_label(e).map_or(String::new(), |(label, amount, _)| format!(
                            "{} {}{amount}",
                            crate::i18n::tr(label),
                            crate::i18n::currency_prefix()
                        )),
                        e.note().as_str()
                    ),
                )
            })
            .collect(),
        _ => vec![],
    }
}
#[component]
pub(crate) fn PromptReview(source: String, drafts: Vec<PromptDraft>, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    let prepared = store.prompt_prepared.read().clone();
    rsx! {
        h2 { {crate::i18n::text("ตรวจ {0} คำสั่งจากข้อความ", &[format!("{}", drafts.len())])} }
        p { class: "model-source", "{source}" }
        if let Some(plan) = prepared {
            p { class: "assistant-message", {crate::i18n::text("ตรวจครบแล้ว ยืนยันเพื่อดำเนินการทุกคำสั่งพร้อมกัน", &[])} }
            for description in &plan.descriptions { div { class: "batch-item prompt-description", "{description}" } }
            if plan.before().accounts.iter().any(|a| !plan.after().accounts.iter().any(|b| b.id() == a.id())) {
                p { class: "batch-question", {crate::i18n::text("คำสั่งนี้มีการลบบัญชีและรายการที่เกี่ยวข้องถาวร ตรวจผลกระทบด้านบนก่อนยืนยัน", &[])} }
            }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.send(Command::CommitPrompt(plan.clone())), {crate::i18n::text("ยืนยันดำเนินการทั้งหมด", &[])} }
            button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.prompt_prepared.set(None), {crate::i18n::text("กลับไปแก้รายละเอียด", &[])} }
        } else {
            p { class: "field-hint", {crate::i18n::text("เติมช่องที่ขาดหรือแก้ชื่อให้ตรง เลือกข้อมูลที่มีอยู่ได้ด้านล่าง หากสร้างบัญชีหรือแผนในข้อความนี้ ให้ใช้ชื่อเดียวกับคำสั่งก่อนหน้า", &[])} }
            for (index, draft) in drafts.iter().enumerate() {
                fieldset { class: "batch-item", disabled: *store.busy.read(),
                    legend { "{index + 1}. {crate::i18n::tr(draft.kind.label())}" }
                    p { class: "field-hint", "{draft.source}" }
                    for question in draft.questions() { p { class: "batch-question", role: "status", "{question}" } }
                    for field in draft.kind.fields() {
                        { let value = draft.get(field.key).to_owned();
                          let choices = options(field.kind, &view);
                          let display = if value.is_empty() { String::new() } else { prompt_display_value(&state_from_dashboard(&view), field.kind, &value) };
                          let key = field.key;
                          rsx! {
                            label { r#for: "prompt-{index}-{key}", "{crate::i18n::tr(field.label)}" if field.required { " *" } }
                            input { id: "prompt-{index}-{key}", value: display,
                                r#type: match field.kind { PromptFieldType::Date => "date", PromptFieldType::Month => "month", _ => "text" },
                                inputmode: if field.kind == PromptFieldType::Money { "decimal" } else { "text" },
                                oninput: move |event| edit(store, index, key, event.value())
                            }
                            if !choices.is_empty() {
                                select { "aria-label": crate::i18n::text("เลือก{0} สำหรับคำสั่งที่ {1}", &[field.label.to_string(), format!("{}", index + 1)]), value: value.clone(),
                                    onchange: move |event| edit(store, index, key, event.value()),
                                    option { value: value.clone(), {crate::i18n::text("เลือกจากรายการที่มีอยู่…", &[])} }
                                    for (id, label) in choices { option { value: id, "{crate::i18n::tr(&label)}" } }
                                }
                            }
                          }
                        }
                    }
                }
            }
            button { class: "primary full-width", disabled: *store.busy.read(), onclick: move |_| store.send(Command::PreviewPrompt(drafts.clone())), {crate::i18n::text("ตรวจทุกคำสั่งก่อนบันทึก", &[])} }
        }
    }
}
