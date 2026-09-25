//! Microphone interaction is presentation state, never a ledger command.
use crate::{Gateway, UiFuture, components::Icon, state::UiState};
use dioxus::dioxus_core::Task;
use dioxus::prelude::*;
use ledger_application::AppError;
use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceSessionId(i64);
impl VoiceSessionId {
    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiceEvent {
    Preparing,
    Permission,
    Listening,
    Processing,
    Finished(String),
    Cancelled,
    ModelRequired,
    ModelDownloadRequested,
    ModelReady,
}

pub(crate) fn voice_unavailable<T: Send + 'static>() -> UiFuture<T> {
    Box::pin(async {
        Err(AppError::Input(
            "รุ่นเดสก์ท็อปยังไม่มีตัวถอดเสียงในเครื่อง พิมพ์รายการได้ตามปกติ".into(),
        ))
    })
}

/// Dropping a page/task must release native capture even during an awaited JNI call.
struct VoiceCapture {
    id: VoiceSessionId,
    gateway: Gateway,
}
impl VoiceCapture {
    fn new(gateway: Gateway) -> Self {
        static NEXT_SESSION: AtomicI64 = AtomicI64::new(1);
        Self {
            id: VoiceSessionId(NEXT_SESSION.fetch_add(1, Ordering::Relaxed)),
            gateway,
        }
    }
}
impl Drop for VoiceCapture {
    fn drop(&mut self) {
        self.gateway.0.cancel_voice(self.id);
    }
}

fn checked_transcript(text: String) -> Result<String, AppError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Input("ยังฟังไม่ชัด ลองพูดอีกครั้งหรือพิมพ์รายการ".into()));
    }
    if text.chars().count() > 1000 {
        // Never truncate an amount, destination, or negation into a different instruction.
        return Err(AppError::Input(
            "ข้อความเสียงยาวเกิน 1,000 ตัวอักษร กรุณาพูดทีละรายการ".into(),
        ));
    }
    Ok(text.to_owned())
}

async fn run_voice_session(
    capture: VoiceCapture,
    download_model: bool,
    mut progress: impl FnMut(VoiceEvent),
) -> Result<VoiceEvent, AppError> {
    if download_model {
        capture
            .gateway
            .0
            .begin_voice_model_download(capture.id)
            .await?;
    } else {
        capture.gateway.0.begin_voice(capture.id).await?;
    }
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(50) {
            return Err(AppError::Input(
                "หมดเวลารับเสียง ลองพูดประโยคสั้น ๆ อีกครั้ง".into(),
            ));
        }
        match capture.gateway.0.poll_voice(capture.id).await? {
            VoiceEvent::Finished(value) => {
                return checked_transcript(value).map(VoiceEvent::Finished);
            }
            event @ (VoiceEvent::Cancelled
            | VoiceEvent::ModelRequired
            | VoiceEvent::ModelDownloadRequested
            | VoiceEvent::ModelReady) => return Ok(event),
            event => progress(event),
        }
    }
}

#[component]
pub(crate) fn VoiceInput(
    text: Signal<String>,
    mut capturing: Signal<bool>,
    #[props(default)] compact: bool,
) -> Element {
    let mut store = use_context::<UiState>();
    let gateway = store.gateway.read().clone();
    let supported = gateway.0.supports_voice();
    let mut task = use_signal(|| None::<Task>);
    let mut session = use_signal(|| None::<VoiceSessionId>);
    let mut phase = use_signal(|| VoiceEvent::Preparing);
    let mut message = use_signal(String::new);
    let mut needs_model = use_signal(|| false);
    let start = use_callback(move |download_model: bool| {
        if *capturing.peek() || *store.busy.peek() {
            return;
        }
        // Dismiss a stale monetary preview before changing its source.
        store.prepared.set(None);
        store.model_choices.set(None);
        store.input.set(None);

        store.guidance.set(String::new());
        capturing.set(true);
        needs_model.set(false);
        phase.set(VoiceEvent::Preparing);
        message.set(String::new());
        let capture = VoiceCapture::new(store.gateway.peek().clone());
        session.set(Some(capture.id));
        task.set(Some(spawn(async move {
            let result = run_voice_session(capture, download_model, |event| phase.set(event)).await;
            session.set(None);
            capturing.set(false);
            match result {
                Ok(VoiceEvent::Finished(value)) => {
                    text.set(value);
                    message.set("ถอดเสียงแล้ว ตรวจจำนวนเงินและชื่อบัญชีในข้อความ แล้วกดอ่านรายการ ยังไม่ได้บันทึก".into());
                }
                Ok(VoiceEvent::ModelRequired) => {
                    needs_model.set(true);
                    message.set("เครื่องนี้รองรับภาษาไทย แต่ยังไม่มีโมเดลออฟไลน์ กดดาวน์โหลดด้านล่างก่อนใช้ครั้งแรก".into());
                }
                Ok(VoiceEvent::ModelDownloadRequested) => message.set("ส่งคำขอดาวน์โหลดให้ Android แล้ว รอระบบติดตั้งให้เสร็จแล้วกดพูดรายการอีกครั้ง ยังไม่ได้เปิดไมโครโฟน".into()),
                Ok(VoiceEvent::ModelReady) => message.set("โมเดลภาษาไทยพร้อมแล้ว กดพูดรายการเพื่อเริ่ม ยังไม่ได้เปิดไมโครโฟน".into()),
                Ok(_) => message.set("หยุดรับเสียงแล้ว ข้อความเดิมยังอยู่".into()),
                Err(error) => message.set(error.to_string()),
            }
        })));
    });
    use_effect(move || {
        let other_action = *store.busy.read()
            || *store.account_form.read()
            || *store.export_form.read()
            || store.input.read().is_some()
            || store.prepared.read().is_some();
        if *capturing.read() && other_action {
            if let Some(task) = task.take() {
                task.cancel();
            }
            session.set(None);
            capturing.set(false);
            message.set("หยุดรับเสียงเพื่อทำรายการอื่น ข้อความเดิมยังอยู่".into());
        }
    });
    let active = *capturing.read();
    let listening = *phase.read() == VoiceEvent::Listening;
    let status = match &*phase.read() {
        VoiceEvent::Preparing => "กำลังตรวจตัวถอดเสียงภาษาไทยในเครื่อง…",
        VoiceEvent::Permission => "อนุญาตไมโครโฟนในหน้าต่างของ Android ก่อนเริ่มพูด",
        VoiceEvent::Listening => "กำลังฟัง… พูดทีละรายการ แล้วเว้นจังหวะหรือกดพูดเสร็จแล้ว",
        VoiceEvent::Processing => "กำลังแปลงเสียงเป็นข้อความในเครื่อง…",
        _ => "",
    };
    rsx! {
        div { class: if compact { "voice-input compact-voice" } else { "voice-input" },
            div { class: "voice-actions",
                if active {
                    if listening {
                        button { r#type: "button", class: "soft-button", onclick: move |_| {
                            if let Some(id) = *session.peek() {
                                store.gateway.peek().0.stop_voice(id);
                                phase.set(VoiceEvent::Processing);
                            }
                        }, Icon { name: "stop", size: 20 } {crate::i18n::tr("พูดเสร็จแล้ว")} }
                    }
                    button { r#type: "button", class: "text-button", onclick: move |_| {
                        if let Some(task) = task.take() { task.cancel(); }
                        session.set(None);
                        capturing.set(false);
                        message.set("ยกเลิกแล้ว ข้อความเดิมยังอยู่".into());
                    }, Icon { name: "close", size: 18 } {crate::i18n::tr("ยกเลิกเสียง")} }
                } else {
                    button { r#type: "button", class: "soft-button voice-button",
                        disabled: !supported || *store.busy.read(),
                        "aria-describedby": "voice-help",
                        title: crate::i18n::tr(if supported { "พูดรายการ" } else { "รุ่นเดสก์ท็อปยังไม่มีตัวถอดเสียงในเครื่อง ทดลองปุ่มไมค์ในแอป Android ที่รองรับได้" }),
                        onclick: move |_| start.call(false), Icon { name: "mic", size: 21 }
                        span { class: if compact { "sr-only" } else { "" }, if text.read().trim().is_empty() { {crate::i18n::tr("พูดรายการ")} } else { {crate::i18n::tr("พูดแทนข้อความ")} } }
                    }
                    if !compact { span { class: "local-badge", {crate::i18n::tr("ภาษาไทย · ในเครื่อง")} } }
                }
            }
            if active {
                p { class: "voice-status", role: "status", "aria-live": "polite", Icon { name: "mic", size: 18 } "{crate::i18n::tr(status)}" }
            } else if !message.read().is_empty() {
                p { class: "voice-status", role: "status", "aria-live": "polite", "{crate::i18n::tr(&message.read())}" }
            }
            if !active && *needs_model.read() {
                button { r#type: "button", class: "soft-button", disabled: *store.busy.read(), onclick: move |_| start.call(true), Icon { name: "download", size: 20 } {crate::i18n::tr("ดาวน์โหลดภาษาไทยออฟไลน์")} }
                p { class: "field-hint", {crate::i18n::tr("ใช้เน็ตเพื่อดาวน์โหลดโมเดลของ Android ครั้งแรก ระบบอาจขอให้ยืนยัน ขนาดขึ้นกับบริการเสียงของเครื่อง")} }
            }
            p { id: "voice-help", class: if compact { "sr-only" } else { "field-hint" },
                if supported {
                    {crate::i18n::tr("เช่น “กาแฟ 80” • ต้องมีตัวถอดเสียงภาษาไทยออฟไลน์ของ Android แอปไม่เก็บไฟล์เสียงและไม่สลับไปใช้ Cloud")}
                } else {
                    {crate::i18n::tr("รุ่นเดสก์ท็อปยังไม่มีตัวถอดเสียงในเครื่อง ทดลองปุ่มไมค์ในแอป Android ที่รองรับได้")}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UiGateway;
    use ledger_application::{Command, CsvExport, ReceiptImage, Response};
    use std::{
        future::{Future, pending},
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize},
        },
        task::{Context, Waker},
    };

    struct SpeechDouble {
        pending_permission: bool,
        event: Option<VoiceEvent>,
        polls: AtomicUsize,
        microphone_requests: AtomicUsize,
        model_requests: AtomicUsize,
        ledger_calls: AtomicUsize,
        cancelled: Mutex<Vec<VoiceSessionId>>,
    }
    impl UiGateway for SpeechDouble {
        fn supports_voice(&self) -> bool {
            true
        }
        fn begin_voice(&self, _: VoiceSessionId) -> UiFuture<()> {
            self.microphone_requests.fetch_add(1, Ordering::SeqCst);
            let wait = self.pending_permission;
            Box::pin(async move { if wait { pending().await } else { Ok(()) } })
        }
        fn poll_voice(&self, _: VoiceSessionId) -> UiFuture<VoiceEvent> {
            let event = self.event.clone();
            self.polls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                if let Some(event) = event {
                    Ok(event)
                } else {
                    pending().await
                }
            })
        }
        fn begin_voice_model_download(&self, _: VoiceSessionId) -> UiFuture<()> {
            self.model_requests.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Ok(()) })
        }
        fn cancel_voice(&self, id: VoiceSessionId) {
            if let Ok(mut cancelled) = self.cancelled.lock() {
                cancelled.push(id);
            }
        }
        fn request(&self, _: Command) -> UiFuture<Response> {
            self.ledger_calls.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(AppError::Input("unexpected ledger command".into())) })
        }
        fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
            Box::pin(async { Ok(None) })
        }
        fn scan_receipt(
            &self,
            _: dioxus::html::FileData,
            _: Arc<AtomicBool>,
        ) -> UiFuture<(ReceiptImage, String)> {
            Box::pin(async { Err(AppError::Input("unexpected OCR call".into())) })
        }
    }
    fn fake(pending_permission: bool, event: Option<VoiceEvent>) -> Arc<SpeechDouble> {
        Arc::new(SpeechDouble {
            pending_permission,
            event,
            polls: AtomicUsize::new(0),
            microphone_requests: AtomicUsize::new(0),
            model_requests: AtomicUsize::new(0),
            ledger_calls: AtomicUsize::new(0),
            cancelled: Mutex::new(Vec::new()),
        })
    }

    #[test]
    fn transcription_is_preserved_for_review_without_changing_the_amount() {
        assert_eq!(
            checked_transcript("  กาแฟ 80.50 บาท  ".into())
                .ok()
                .as_deref(),
            Some("กาแฟ 80.50 บาท")
        );
        assert_eq!(
            checked_transcript("โอน แปดสิบ ไม่ใช่แปดร้อย".into())
                .ok()
                .as_deref(),
            Some("โอน แปดสิบ ไม่ใช่แปดร้อย")
        );
    }

    #[test]
    fn empty_or_overlong_speech_is_rejected_without_truncation() {
        assert!(checked_transcript(" \n ".into()).is_err());
        assert!(checked_transcript(format!("{}800", "ก".repeat(998))).is_err());
        assert!(checked_transcript("ก".repeat(1000)).is_ok());
    }

    #[tokio::test]
    async fn a_finished_transcription_returns_only_draft_text_and_releases_capture() {
        let adapter = fake(false, Some(VoiceEvent::Finished("กาแฟ 80".into())));
        let capture = VoiceCapture::new(Gateway(adapter.clone()));
        let id = capture.id;
        let result = run_voice_session(capture, false, |_| {}).await;
        assert_eq!(result.ok(), Some(VoiceEvent::Finished("กาแฟ 80".into())));
        assert_eq!(adapter.ledger_calls.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.cancelled.lock().ok().as_deref(), Some(&vec![id]));
    }

    #[test]
    fn leaving_the_page_cancels_during_permission_or_result_wait() {
        for waiting_permission in [true, false] {
            let adapter = fake(waiting_permission, None);
            let capture = VoiceCapture::new(Gateway(adapter.clone()));
            let id = capture.id;
            let mut future = Box::pin(run_voice_session(capture, false, |_| {}));
            assert!(
                future
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            drop(future);
            assert_eq!(adapter.cancelled.lock().ok().as_deref(), Some(&vec![id]));
            assert_eq!(adapter.ledger_calls.load(Ordering::SeqCst), 0);
            assert_eq!(
                adapter.polls.load(Ordering::SeqCst),
                usize::from(!waiting_permission)
            );
        }
    }

    #[tokio::test]
    async fn cancellation_does_not_return_partial_text() {
        let adapter = fake(false, Some(VoiceEvent::Cancelled));
        assert!(matches!(
            run_voice_session(VoiceCapture::new(Gateway(adapter)), false, |_| {}).await,
            Ok(VoiceEvent::Cancelled)
        ));
    }

    #[test]
    fn restarted_captures_have_distinct_ids_for_ignoring_stale_callbacks() {
        let gateway = Gateway(fake(false, None));
        let first = VoiceCapture::new(gateway.clone());
        let second = VoiceCapture::new(gateway);
        assert_ne!(first.id, second.id);
    }

    #[tokio::test]
    async fn downloading_a_model_does_not_start_the_microphone_or_write_a_ledger() {
        let adapter = fake(false, Some(VoiceEvent::ModelDownloadRequested));
        let result =
            run_voice_session(VoiceCapture::new(Gateway(adapter.clone())), true, |_| {}).await;
        assert_eq!(result.ok(), Some(VoiceEvent::ModelDownloadRequested));
        assert_eq!(adapter.microphone_requests.load(Ordering::SeqCst), 0);
        assert_eq!(adapter.model_requests.load(Ordering::SeqCst), 1);
        assert_eq!(adapter.ledger_calls.load(Ordering::SeqCst), 0);
    }
}
