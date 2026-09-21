use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use std::sync::atomic::Ordering;

#[component]
pub(crate) fn ModelProgress() -> Element {
    let store = use_context::<UiState>();
    let mut tick = use_signal(|| 0_u64);
    let started = use_hook(std::time::Instant::now);
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            tick += 1;
        }
    });
    let _tick = tick();
    let active = store.model_operation.read().clone();
    let Some(active) = active else {
        return rsx! {};
    };
    let stage = match active.phase() {
        ModelPhase::Queued => "กำลังเตรียมอ่านรายการ",
        ModelPhase::Verifying => "กำลังตรวจไฟล์ AI",
        ModelPhase::Loading => "กำลังเปิด AI ในเครื่อง",
        ModelPhase::Reading => "AI กำลังอ่านข้อความ",
        ModelPhase::Generating => "AI กำลังจัดรายละเอียดรายการ",
    };
    let elapsed = started.elapsed().as_secs();
    let tokens = active.generated_tokens.load(Ordering::Relaxed);
    rsx! {
        div { class: "model-progress", role: "status",
            div { class: "progress-heading", span { class: "progress-spinner", "aria-hidden": "true" } strong { "{stage}" } }
            p { "{elapsed} วินาที · ประมวลผลบนเครื่องนี้" }
            if tokens > 0 { small { "กำลังเรียบเรียงคำตอบ…" } }
            if elapsed >= 10 { p { class: "field-hint", "ครั้งแรกอาจใช้เวลาสักครู่ โดยเฉพาะบน Emulator ยกเลิกแล้วกรอกเองได้" } }
            button { class: "soft-button", onclick: move |_| store.cancel_model(), "ยกเลิกการอ่านรายการ" }
            button { class: "text-button", onclick: move |_| store.new_entry(), Icon { name: "edit", size: 20 } "ยกเลิกแล้วกรอกเอง" }
        }
    }
}

#[component]
pub(crate) fn ModelSettings(capturing: Signal<bool>) -> Element {
    let mut store = use_context::<UiState>();
    let mut installed = use_signal(|| false);
    let mut checking = use_signal(|| true);
    let mut operation = use_signal(|| None::<ModelOperation>);
    let mut message = use_signal(String::new);
    use_future(move || async move {
        let gateway = store.gateway.peek().clone();
        match gateway.0.model_availability().await {
            Ok(ModelAvailability::Installed) => installed.set(true),
            Ok(ModelAvailability::Missing) => {}
            Err(error) => message.set(error.to_string()),
        }
        checking.set(false);
    });
    use_drop(move || {
        if let Some(active) = operation.peek().as_ref() {
            active.cancelled.store(true, Ordering::Relaxed);
        }
    });
    rsx! {
        div { class: "model-settings",
            strong { if *installed.read() { "AI ในเครื่องพร้อมใช้" } else if *checking.read() { "กำลังตรวจ AI ในเครื่อง…" } else { "เปิดใช้ AI สำหรับข้อความอิสระ" } }
            p { class: "field-hint", "ข้อความบัญชีประมวลผลบนเครื่อง ดาวน์โหลดโมเดลครั้งแรกประมาณ 397 MB หลังจากนั้นใช้ได้ออฟไลน์" }
            if !*installed.read() {
                if operation.read().is_some() {
                    p { role: "status", "กำลังดาวน์โหลดและตรวจไฟล์ AI…" }
                    button { class: "text-button", onclick: move |_| { if let Some(active) = operation.peek().as_ref() { active.cancelled.store(true, Ordering::Relaxed); message.set("กำลังยกเลิกการดาวน์โหลด…".into()); } }, "ยกเลิกดาวน์โหลด" }
                } else {
                    button { class: "soft-button", disabled: *checking.read() || *capturing.read() || *store.busy.read(), onclick: move |_| {
                        let active = ModelOperation::default();
                        operation.set(Some(active.clone()));
                        store.busy.set(true);
                        message.set(String::new());
                        let gateway = store.gateway.peek().clone();
                        // Root-owned cleanup clears global busy even if this page unmounts.
                        // Component signals are only touched while mounted via try_write.
                        dioxus::dioxus_core::spawn_forever(async move {
                            let result = gateway.0.install_model(active).await;
                            if let Ok(mut value) = installed.try_write() { *value = result.is_ok(); }
                            if let Ok(mut value) = message.try_write() { *value = match result { Ok(()) => "ดาวน์โหลด AI พร้อมใช้แล้ว".into(), Err(error) => error.to_string() }; }
                            if let Ok(mut value) = operation.try_write() { *value = None; }
                            store.busy.set(false);
                        });
                    }, "ดาวน์โหลด AI ในเครื่อง" }
                }
            }
            if !message.read().is_empty() { p { class: "field-hint", role: "status", "{message}" } }
            details { class: "model-licenses",
                summary { "โมเดลและสัญญาอนุญาต" }
                p { "Qwen3 0.6B · Unsloth Q4_K_M · ประมวลผลในเครื่องด้วย llama.cpp" }
                pre { {include_str!("../../../licenses/local-ai/Qwen3-APACHE-2.0.txt")} }
                pre { {include_str!("../../../licenses/local-ai/llama-cpp-rs-MIT.txt")} }
                pre { {include_str!("../../../licenses/local-ai/llama-cpp-MIT.txt")} }
            }
        }
    }
}

#[component]
pub(crate) fn ModelChoices(source: String, drafts: Vec<ModelDraft>, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    rsx! {
        h2 { "AI เข้าใจตรงกับเราไหม?" }
        p { class: "model-source", "{source}" }
        p { class: "field-hint", "เลือกความหมายที่ต้องการ แล้วเติมรายละเอียดและตรวจอีกครั้งก่อนบันทึก" }
        for (index, draft) in drafts.into_iter().enumerate() {
            { let kind = match draft.input.kind { TransactionKind::Expense => "จ่าย", TransactionKind::Income => "รับ", TransactionKind::Transfer => "โอน" };
              let account = draft.input.account.map(|id| account_label(&view, id)).unwrap_or_else(|| "เลือกบัญชี".into());
              let destination = draft.input.destination.map(|id| account_label(&view, id));
              let amount = if draft.input.amount.is_empty() { "ยังไม่ทราบยอด".into() } else { format!("{} บาท", draft.input.amount) };
              let category = draft.input.category.map(|c| c.label().to_owned()).unwrap_or_else(|| "เลือกหมวด".into());
              rsx! { div { class: "model-choice", key: "{index}",
                strong { "{kind} · {amount}" }
                p { "{account}" if let Some(destination) = destination { " → {destination}" } }
                if draft.input.kind != TransactionKind::Transfer { p { "{category}" } }
                p { "วันที่: {draft.input.date}" }
                if !draft.input.note.is_empty() { p { "{draft.input.note}" } }
                p { class: "field-hint", "{draft.guidance}" }
                button { class: "soft-button", disabled: *store.busy.read(), onclick: move |_| {
                    store.input.set(Some(draft.input.clone()));
                    store.guidance.set(draft.guidance.clone());
                    store.model_choices.set(None);
                    store.prepared.set(None);
                }, "เลือกและตรวจรายละเอียด" }
              } }
            }
        }
        button { class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.new_entry(), "ไม่ตรง · กรอกเอง" }
    }
}
