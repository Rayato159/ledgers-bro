use crate::{components::*, state::UiState};
use dioxus::prelude::*;
use ledger_application::*;
use std::sync::atomic::Ordering;

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;

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
            p { {crate::i18n::text("{0} วินาที · ประมวลผลบนเครื่องนี้", &[format!("{}", elapsed)])} }
            if tokens > 0 { small { {crate::i18n::text("กำลังเรียบเรียงคำตอบ…", &[])} } }
            if elapsed >= 10 { p { class: "field-hint", {crate::i18n::text("ครั้งแรกอาจใช้เวลาสักครู่ โดยเฉพาะบน Emulator ยกเลิกแล้วกรอกเองได้", &[])} } }
            button { r#type: "button", class: "soft-button", onclick: move |_| store.cancel_model(), {crate::i18n::text("ยกเลิกการอ่านรายการ", &[])} }
            button { r#type: "button", class: "text-button", onclick: move |_| store.new_entry(), Icon { name: "edit", size: 20 } {crate::i18n::text("ยกเลิกแล้วกรอกเอง", &[])} }
        }
    }
}

fn gib(bytes: u64) -> String {
    format!("{:.1} GiB", bytes as f64 / 1_073_741_824.0)
}
fn optional_gib(value: Option<u64>) -> String {
    value.map(gib).unwrap_or_else(|| "—".into())
}
fn fit_label(fit: ModelFit) -> &'static str {
    match fit {
        ModelFit::FitsEstimate => "หน่วยความจำเพียงพอตามค่าประเมิน",
        ModelFit::LowMemory => "RAM ว่างตอนนี้น้อยไป ควรปิดแอปอื่นหรือเลือกโมเดลเล็กลง",
        ModelFit::InsufficientMemory => "RAM ทั้งเครื่องไม่พอสำหรับโมเดลนี้ เลือกตัวที่เล็กลง",
        ModelFit::InsufficientDisk => "พื้นที่ว่างไม่พอสำหรับดาวน์โหลดโมเดลนี้",
        ModelFit::MobileCaution => "มือถืออาจจำกัด RAM ต่อแอป แนะนำเริ่มที่ 0.6B หรือ 1.7B",
        ModelFit::Unknown => "อ่านข้อมูลเครื่องได้ไม่ครบ จึงยังยืนยันความเหมาะสมไม่ได้",
    }
}

/// Session-owned state: switching tabs must not cancel installation or lose progress.
#[derive(Clone, Copy)]
pub(crate) struct ModelLibrary {
    snapshot: Signal<Option<ModelSettingsSnapshot>>,
    selected: Signal<LocalModelId>,
    checking: Signal<bool>,
    operation: Signal<Option<ModelOperation>>,
    message: Signal<String>,
}

pub(crate) fn use_model_library() -> ModelLibrary {
    let library = ModelLibrary {
        snapshot: use_signal(|| None),
        selected: use_signal(|| LocalModelId::Small),
        checking: use_signal(|| false),
        operation: use_signal(|| None),
        message: use_signal(String::new),
    };
    use_context_provider(|| library);
    use_drop(move || {
        if let Some(active) = library.operation.peek().as_ref() {
            active.cancelled.store(true, Ordering::Relaxed);
        }
    });
    library
}

impl ModelLibrary {
    fn refresh(mut self, store: UiState) {
        if *self.checking.peek() || self.operation.peek().is_some() || *store.busy.peek() {
            return;
        }
        self.checking.set(true);
        let gateway = store.gateway.peek().clone();
        crate::state::spawn_session(async move {
            match gateway.0.model_settings().await {
                Ok(value) => {
                    if self.snapshot.peek().is_none() {
                        self.selected.set(value.selected);
                    }
                    self.snapshot.set(Some(value));
                }
                Err(error) => self.message.set(error.to_string()),
            }
            self.checking.set(false);
        });
    }

    fn activate(mut self, mut store: UiState, id: LocalModelId) {
        if *store.busy.peek() || *self.checking.peek() || self.operation.peek().is_some() {
            return;
        }
        let active = ModelOperation::default();
        self.selected.set(id);
        self.operation.set(Some(active.clone()));
        store.busy.set(true);
        self.message.set(String::new());
        let gateway = store.gateway.peek().clone();
        crate::state::spawn_session(async move {
            let result = gateway.0.activate_model(id, active).await;
            match result {
                Ok(()) => {
                    self.message
                        .set(crate::i18n::tr("ตรวจไฟล์แล้ว เปลี่ยนโมเดลเรียบร้อย"));
                    match gateway.0.model_settings().await {
                        Ok(value) => {
                            self.selected.set(value.selected);
                            self.snapshot.set(Some(value));
                        }
                        Err(error) => self.message.set(error.to_string()),
                    }
                }
                Err(error) => self.message.set(error.to_string()),
            }
            self.operation.set(None);
            store.busy.set(false);
        });
    }

    fn cancel(mut self) {
        if let Some(active) = self.operation.peek().as_ref() {
            active.cancelled.store(true, Ordering::Relaxed);
        }
        self.message.set(crate::i18n::tr("กำลังยกเลิกการดาวน์โหลด…"));
    }
}

#[component]
pub(crate) fn ModelSettings(
    capturing: Signal<bool>,
    #[props(default = false)] allow_delete: bool,
    #[props(default = false)] compact: bool,
) -> Element {
    let mut store = use_context::<UiState>();
    let library = use_context::<ModelLibrary>();
    let mut snapshot = library.snapshot;
    let mut selected = library.selected;
    let checking = library.checking;
    let operation = library.operation;
    let mut message = library.message;
    let mut confirmation = use_signal(|| false);
    use_hook(move || library.refresh(store));
    let spec = selected().info();
    let data = snapshot();
    let downloaded = data
        .as_ref()
        .is_some_and(|s| s.downloaded.contains(&selected()));
    let action_icon = if downloaded { "check" } else { "download" };
    let fit = data
        .as_ref()
        .map(|s| model_fit(&spec, &s.device, !downloaded))
        .unwrap_or(ModelFit::Unknown);
    rsx! {
        div { class: if compact { "model-settings compact-model" } else { "model-settings" },
            if !compact {
            h2 { {crate::i18n::tr("เลือก AI ในเครื่อง")} }
            p { class: "field-hint", {crate::i18n::tr("ข้อความประมวลผลในเครื่อง โมเดลที่เลือกใช้ร่วมกันทุกผู้ใช้บนอุปกรณ์นี้")} }
            if let Some(data) = &data {
                p { {crate::i18n::tr("โมเดลที่ใช้อยู่")} strong { " · {data.selected.info().name}" } }
            }
            }
            label { class: if compact { "sr-only" } else { "" }, r#for: "local-model-choice", {crate::i18n::tr("โมเดล")} }
            select { id: "local-model-choice", value: selected().code(), disabled: checking() || operation.read().is_some() || *store.busy.read(),
                onchange: move |e| { if let Some(id) = LocalModelId::from_code(&e.value()) { selected.set(id); message.set(String::new()); } },
                for id in LocalModelId::ALL { option { value: id.code(), "{id.info().name}" } }
            }
            if !compact { div { class: "model-device-summary",
                p { {crate::i18n::tr("ขนาดดาวน์โหลด")} strong { "{gib(spec.bytes)}" } }
                p { {crate::i18n::tr("RAM ที่โมเดลต้องใช้โดยประมาณ")} strong { "{gib(spec.working_memory_bytes)}" } }
                if let Some(data) = &data {
                    p { {crate::i18n::tr("RAM ทั้งเครื่อง / ว่างตอนนี้")} strong { "{optional_gib(data.device.total_memory)} / {optional_gib(data.device.available_memory)}" } }
                    p { {crate::i18n::tr("พื้นที่ว่างสำหรับโมเดล")} strong { "{optional_gib(data.device.free_disk)}" } }
                }
            }
            if compact && let Some(data) = &data && data.selected != selected() {
                p { class: "compact-model-note", {crate::i18n::tr("โมเดลที่ใช้อยู่")} " · {data.selected.info().name}" }
            }
            if compact && fit.blocked() {
                p { class: "compact-model-note model-fit-note", "data-warning": true, {crate::i18n::tr(fit_label(fit))} }
            }
            p { class: "model-fit-note", "data-warning": fit != ModelFit::FitsEstimate, {crate::i18n::tr(fit_label(fit))} }
            p { class: "field-hint", {crate::i18n::tr("รุ่นนี้ประมวลผลด้วย CPU โมเดลใหญ่จะช้าลง ค่าหน่วยความจำเป็นการประเมิน ไม่ใช่การรับประกันความเร็วหรือความแม่นยำ")} }
            }
            if let Some(active) = operation.read().clone() {
                p { class: "field-hint", {crate::i18n::tr("เปลี่ยนแท็บได้ การดาวน์โหลดจะทำงานต่อจนกว่าจะเสร็จหรือกดยกเลิก")} }
                ModelDownloadProgress { operation: active, total: spec.bytes }
                button { r#type: "button", class: "soft-button", onclick: move |_| library.cancel(), {crate::i18n::tr("ยกเลิกดาวน์โหลด")} }
            } else {
                div { class: "model-settings-actions",
                    button { r#type: "button", class: if compact { "icon-button" } else { "primary" }, title: crate::i18n::tr(if downloaded { "ตรวจไฟล์และใช้โมเดลนี้" } else { "ดาวน์โหลดและใช้โมเดลนี้" }), disabled: checking() || data.is_none() || fit.blocked() || *capturing.read() || *store.busy.read(), onclick: move |_| confirmation.set(true),
                        if compact { Icon { name: action_icon, size: 18 } }
                        span { class: if compact { "sr-only" } else { "" },
                        if downloaded { {crate::i18n::tr("ตรวจไฟล์และใช้โมเดลนี้")} } else { {crate::i18n::tr("ดาวน์โหลดและใช้โมเดลนี้")} } }
                    }
                    if !compact { button { r#type: "button", class: "soft-button", disabled: checking() || *store.busy.read(), onclick: move |_| { library.refresh(store); }, {crate::i18n::tr("ตรวจเครื่องอีกครั้ง")} } }
                }
            }
            if checking() { p { role: "status", {crate::i18n::tr("กำลังตรวจ AI ในเครื่อง…")} } }
            if allow_delete && let Some(data) = &data {
                section { class: "model-library",
                    h3 { {crate::i18n::tr("โมเดลที่ดาวน์โหลดแล้ว")} }
                    p { class: "field-hint", {crate::i18n::tr("ลบเฉพาะไฟล์โมเดล ข้อมูลบัญชียังอยู่ ถ้าลบโมเดลที่ใช้อยู่ให้เลือกโมเดลอื่นหรือดาวน์โหลดใหม่")} }
                    for id in data.downloaded.iter().copied() {
                        div { class: "setting-row", strong { "{id.info().name}" }
                            button { r#type: "button", class: "danger-button", disabled: *store.busy.read(), onclick: move |_| {
                                crate::confirmation::ask(format!("{}: {}", crate::i18n::tr("ลบโมเดล"), id.info().name), move |_| {
                                    store.busy.set(true);
                                    let gateway = store.gateway.peek().clone();
                                    crate::state::spawn_session(async move {
                                        let result = gateway.0.delete_model(id).await;
                                        if result.is_ok() && let Ok(value) = gateway.0.model_settings().await && let Ok(mut target) = snapshot.try_write() { *target = Some(value); }
                                        if let Ok(mut target) = message.try_write() { *target = match result { Ok(()) => crate::i18n::tr("ลบโมเดลแล้ว"), Err(e) => e.to_string() }; }
                                        store.busy.set(false);
                                    });
                                });
                            }, Icon { name: "trash", size: 18 } {crate::i18n::tr("ลบโมเดล")} }
                        }
                    }
                }
            }
            if !message.read().is_empty() { p { class: "field-hint", role: "status", "{message}" } }
            if !compact { details { class: "model-licenses", summary { {crate::i18n::tr("โมเดลและสัญญาอนุญาต")} }
                p { "Qwen3 · Q4_K_M · Apache-2.0 · llama.cpp (MIT)" }
                a { href: spec.source, target: "_blank", rel: "noopener noreferrer", {crate::i18n::tr("ข้อมูลและสัญญาอนุญาตจากผู้เผยแพร่โมเดล")} }
            }
        }
        }
        if confirmation() {
            dialog { id: "model-install-dialog", class: "account-dialog model-install-dialog", "aria-labelledby": "model-install-title",
                onmounted: move |_| { let _ = document::eval("document.getElementById('model-install-dialog').showModal()"); },
                oncancel: move |e| { e.prevent_default(); confirmation.set(false); },
                h2 { id: "model-install-title", "{spec.name}" }
                p { {crate::i18n::tr("ขนาดดาวน์โหลด")} ": {gib(spec.bytes)}" }
                p { {crate::i18n::tr("RAM ที่โมเดลต้องใช้โดยประมาณ")} ": {gib(spec.working_memory_bytes)}" }
                p { class: "model-fit-note", "data-warning": fit != ModelFit::FitsEstimate, {crate::i18n::tr(fit_label(fit))} }
                p { {crate::i18n::tr("โมเดลเดิมจะยังอยู่ การเปลี่ยน AI ไม่เปลี่ยนข้อมูลบัญชี และต้องตรวจรายการก่อนบันทึกเสมอ")} }
                div { class: "model-settings-actions",
                    button { r#type: "button", class: "primary", disabled: *store.busy.read() || fit.blocked(), onclick: move |_| {
                        confirmation.set(false);
                        library.activate(store, selected());
                    }, {crate::i18n::tr("ยืนยันใช้โมเดลนี้")} }
                    button { r#type: "button", class: "soft-button", onclick: move |_| confirmation.set(false), {crate::i18n::tr("ยกเลิก")} }
                }
            }
        }
    }
}

#[component]
fn ModelDownloadProgress(operation: ModelOperation, total: u64) -> Element {
    let mut tick = use_signal(|| 0u64);
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            tick += 1;
        }
    });
    let _tick = tick();
    let done = operation.downloaded_bytes.load(Ordering::Relaxed);
    rsx! { div { class: "model-download-progress", role: "status",
        p { {crate::i18n::tr("กำลังดาวน์โหลดและตรวจไฟล์ AI…")} }
        progress { max: "{total}", value: "{done}", "aria-label": crate::i18n::tr("ดาวน์โหลดโมเดล") }
        p { "{gib(done)} / {gib(total)}" }
    } }
}

#[component]
pub(crate) fn ModelChoices(source: String, drafts: Vec<ModelDraft>, view: Dashboard) -> Element {
    let mut store = use_context::<UiState>();
    rsx! {
        h2 { {crate::i18n::text("AI เข้าใจตรงกับเราไหม?", &[])} }
        p { class: "model-source", "{source}" }
        p { class: "field-hint", {crate::i18n::text("เลือกความหมายที่ต้องการ แล้วเติมรายละเอียดและตรวจอีกครั้งก่อนบันทึก", &[])} }
        for (index, draft) in drafts.into_iter().enumerate() {
            { let kind = match draft.input.kind { TransactionKind::Expense => "จ่าย", TransactionKind::Income => "รับ", TransactionKind::Transfer => "โอน" };
              let account = draft.input.account.map(|id| account_label(&view, id)).unwrap_or_else(|| "เลือกบัญชี".into());
              let destination = draft.input.destination.map(|id| account_label(&view, id));
              let amount = if draft.input.amount.is_empty() { {crate::i18n::text("ยังไม่ทราบยอด", &[])} } else { format!("{} บาท", draft.input.amount) };
              let category = draft.input.category.map(|c| crate::i18n::tr(c.label()).to_owned()).unwrap_or_else(|| "เลือกหมวด".into());
              rsx! { div { class: "model-choice", key: "{index}",
                strong { "{kind} · {amount}" }
                p { "{account}" if let Some(destination) = destination { " → {destination}" } }
                if draft.input.kind != TransactionKind::Transfer { p { "{category}" } }
                p { {crate::i18n::text("วันที่: {0}", std::slice::from_ref(&draft.input.date))} }
                if !draft.input.note.is_empty() { p { "{draft.input.note}" } }
                p { class: "field-hint", "{draft.guidance}" }
                button { r#type: "button", class: "soft-button", disabled: *store.busy.read(), onclick: move |_| {
                    store.input.set(Some(draft.input.clone()));
                    store.guidance.set(draft.guidance.clone());
                    store.model_choices.set(None);
                    store.prepared.set(None);
                }, {crate::i18n::text("เลือกและตรวจรายละเอียด", &[])} }
              } }
            }
        }
        button { r#type: "button", class: "text-button", disabled: *store.busy.read(), onclick: move |_| store.new_entry(), {crate::i18n::text("ไม่ตรง · กรอกเอง", &[])} }
    }
}
