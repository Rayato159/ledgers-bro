use crate::Gateway;
use crate::receipt::ReceiptReview;
use dioxus::prelude::*;
use ledger_application::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Accounts,
    Transactions,
    Chat,
    Manual,
    Tax,
}
impl Page {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Accounts,
        Self::Chat,
        Self::Transactions,
        Self::Tax,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "ภาพรวม",
            Self::Accounts => "บัญชี",
            Self::Transactions => "รายการ",
            Self::Chat => "บันทึกด่วน",
            Self::Manual => "กรอกเอง",
            Self::Tax => "ภาษี",
        }
    }
    pub const fn icon(self) -> &'static str {
        match self {
            Self::Overview => "home",
            Self::Accounts => "wallet",
            Self::Transactions => "list",
            Self::Chat => "chat",
            Self::Manual => "edit",
            Self::Tax => "file",
        }
    }
}

#[derive(Clone, Copy)]
pub struct UiState {
    pub gateway: Signal<Gateway>,
    pub view: Signal<Option<Dashboard>>,
    pub page: Signal<Page>,
    pub busy: Signal<bool>,
    pub notice: Signal<Option<(bool, String)>>,
    pub account_form: Signal<bool>,
    pub export_form: Signal<bool>,
    pub account_deletion: Signal<Option<AccountDeletion>>,
    pub input: Signal<Option<EntryInput>>,
    pub prepared: Signal<Option<PreparedEntry>>,
    pub guidance: Signal<String>,
    pub receipt: Signal<Option<ReceiptReview>>,
    pub scan_cancel: Signal<Option<Arc<AtomicBool>>>,
    pub model_operation: Signal<Option<ModelOperation>>,
    pub model_choices: Signal<Option<(String, Vec<ModelDraft>)>>,
}
impl UiState {
    pub fn cancel_model(self) {
        if let Some(operation) = self.model_operation.peek().as_ref() {
            operation.cancelled.store(true, Ordering::Relaxed);
        }
    }
    pub fn resolve_text(mut self, text: String) {
        if *self.busy.peek() {
            return;
        }
        let operation = ModelOperation::default();
        self.model_operation.set(Some(operation.clone()));
        self.model_choices.set(None);
        self.input.set(None);
        self.prepared.set(None);
        self.receipt.set(None);
        self.guidance.set("กำลังอ่านรายการในเครื่อง…".into());
        self.notice.set(None);
        self.busy.set(true);
        let gateway = self.gateway.peek().clone();
        dioxus::dioxus_core::spawn_forever(async move {
            let result = await_model(
                gateway.0.resolve_text(text, operation.clone()),
                &operation,
                std::time::Duration::from_secs(90),
            )
            .await;
            match result {
                Ok(None) => self.guidance.set("ยกเลิกแล้ว ยังไม่มีรายการถูกบันทึก".into()),
                result => {
                    self.guidance.set(String::new());
                    match result {
                        Ok(Some(Response::Resolved(resolution))) => {
                            self.apply_resolution(resolution)
                        }
                        Err(error) => self.notice.set(Some((true, error.to_string()))),
                        _ => self.notice.set(Some((true, "อ่านรายการไม่สำเร็จ".into()))),
                    }
                }
            }
            self.model_operation.set(None);
            self.busy.set(false);
        });
    }
    fn apply_resolution(mut self, resolution: QuickResolution) {
        self.receipt.set(None);
        self.prepared.set(None);
        self.model_choices.set(None);
        match resolution {
            QuickResolution::Draft { input, guidance } => {
                self.input.set(Some(input));
                self.guidance.set(guidance);
            }
            QuickResolution::Choices { source, drafts } => {
                self.input.set(None);
                self.model_choices.set(Some((source, drafts)));
            }
            QuickResolution::Summary => self.page.set(Page::Overview),
            QuickResolution::Help => self
                .guidance
                .set("ใช้ตัวอย่างด้านบน หรือเลือกแบบฟอร์ม กรอกชื่อที่มีช่องว่างในเครื่องหมายคำพูด".into()),
        }
    }
    pub fn scan_receipt(self, file: dioxus::html::FileData) {
        self.run_receipt_scan(Some(file));
    }
    pub fn pick_receipt(self) {
        self.run_receipt_scan(None);
    }
    fn run_receipt_scan(mut self, file: Option<dioxus::html::FileData>) {
        if *self.busy.peek() {
            return;
        }
        if file
            .as_ref()
            .is_some_and(|file| file.size() > MAX_RECEIPT_BYTES as u64)
        {
            self.notice
                .set(Some((true, "รูปใบเสร็จต้องไม่เกิน 32 MB".into())));
            return;
        }
        let Some(today) = self.view.peek().as_ref().map(|view| view.today) else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.scan_cancel.set(Some(cancel.clone()));
        self.model_choices.set(None);
        self.busy.set(true);
        self.notice.set(None);
        let gateway = self.gateway.peek().clone();
        dioxus::dioxus_core::spawn_forever(async move {
            let result = if let Some(file) = file {
                gateway.0.scan_receipt(file, cancel.clone()).await.map(Some)
            } else {
                gateway.0.pick_receipt(cancel.clone()).await
            };
            if cancel.load(Ordering::Relaxed) {
                self.notice
                    .set(Some((false, "ยกเลิกการอ่านใบเสร็จแล้ว".into())));
            } else {
                match result.and_then(|result| {
                    result
                        .map(|(image, text)| {
                            analyze_receipt(&text, today).map(|analysis| (image, analysis))
                        })
                        .transpose()
                }) {
                    Ok(Some((image, analysis))) => {
                        self.input.set(Some(analysis.draft(today)));
                        self.prepared.set(None);
                        self.receipt.set(Some(ReceiptReview::new(image, analysis)));
                        self.guidance
                            .set("อ่านใบเสร็จแล้ว ตรวจยอด วันที่ และเลือกบัญชีกับหมวดก่อนบันทึก".into());
                        self.page.set(Page::Chat);
                    }
                    Ok(None) => {}
                    Err(error) => self.notice.set(Some((true, error.to_string()))),
                }
            }
            self.scan_cancel.set(None);
            self.busy.set(false);
        });
    }

    pub fn cancel_scan(self) {
        if let Some(cancel) = self.scan_cancel.peek().as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn send(mut self, command: Command) {
        if *self.busy.peek() {
            return;
        }
        self.busy.set(true);
        self.notice.set(None);
        let gateway = self.gateway.peek().clone();
        // A successful commit dismisses its dialog/preview. The task must belong
        // to the root, otherwise unmounting that child cancels the refresh and
        // leaves the entire app busy even though the transaction was saved.
        dioxus::dioxus_core::spawn_forever(async move {
            let result = gateway.0.request(command).await;
            match result {
                Ok(Response::Dashboard(view)) => self.view.set(Some(view)),
                Ok(Response::Resolved(resolution)) => self.apply_resolution(resolution),
                Ok(Response::Prepared(prepared)) => self.prepared.set(Some(prepared)),
                Ok(Response::AccountDeletion(deletion)) => {
                    self.account_deletion.set(Some(deletion))
                }
                Ok(Response::AccountDeleted(id)) => {
                    self.account_deletion.set(None);
                    // A draft referring to the removed account cannot be saved.
                    let uses_deleted = self.input.peek().as_ref().is_some_and(|input| {
                        input.account == Some(id) || input.destination == Some(id)
                    });
                    if uses_deleted {
                        self.prepared.set(None);
                        if let Some(input) = self.input.write().as_mut() {
                            if input.account == Some(id) {
                                input.account = None;
                            }
                            if input.destination == Some(id) {
                                input.destination = None;
                            }
                        }
                    }
                    // Never leave a stale dashboard available if refresh fails.
                    self.view.set(None);
                    match gateway.0.request(Command::Load).await {
                        Ok(Response::Dashboard(view)) => {
                            self.view.set(Some(view));
                            self.notice
                                .set(Some((false, "ลบบัญชีและรายการที่เกี่ยวข้องแล้ว".into())));
                        }
                        _ => self.notice.set(Some((
                            true,
                            "ลบแล้ว แต่โหลดภาพรวมไม่สำเร็จ กรุณากดโหลดใหม่ก่อนทำรายการต่อ".into(),
                        ))),
                    }
                }
                Ok(Response::Committed(_)) => {
                    self.receipt.set(None);
                    self.account_form.set(false);
                    self.prepared.set(None);
                    self.input.set(None);
                    match gateway.0.request(Command::Load).await {
                        Ok(Response::Dashboard(view)) => {
                            self.view.set(Some(view));
                            self.notice.set(Some((false, "บันทึกในเครื่องแล้ว".into())));
                        }
                        _ => self.notice.set(Some((
                            true,
                            "บันทึกแล้ว แต่โหลดภาพรวมไม่สำเร็จ กรุณากดโหลดใหม่ก่อนทำรายการต่อ".into(),
                        ))),
                    }
                }
                Ok(Response::Csv(csv)) => match gateway.0.save_csv(csv).await {
                    Ok(Some(name)) => {
                        self.export_form.set(false);
                        self.notice.set(Some((false, format!("ส่งออกแล้ว: {name}"))));
                    }
                    Ok(None) => {}
                    Err(error) => self.notice.set(Some((true, error.to_string()))),
                },
                Err(error) => self.notice.set(Some((true, error.to_string()))),
            }
            self.busy.set(false);
        });
    }
    pub fn new_entry(mut self) {
        if *self.busy.peek() && self.model_operation.peek().is_none() {
            return;
        }
        let today = self.view.peek().as_ref().map(|v| v.today);
        if let Some(today) = today {
            // Cancel interpretation before opening an independent draft. The
            // awaiting task rejects late model output and releases busy itself.
            self.cancel_model();
            self.model_choices.set(None);
            self.receipt.set(None);
            self.input.set(Some(EntryInput::empty(today)));
            self.prepared.set(None);
            self.guidance.set(String::new());
            self.notice.set(None);
            self.page.set(Page::Manual);
        }
    }
    pub fn update_entry(mut self, update: impl FnOnce(&mut EntryInput)) {
        if *self.busy.peek() {
            return;
        }
        self.prepared.set(None);
        if let Some(input) = self.input.write().as_mut() {
            let previous_amount = input.amount.clone();
            update(input);
            if input.amount != previous_amount
                && let Some(receipt) = &mut input.receipt
            {
                receipt.reviewed = false;
            }
        }
    }
    pub fn update_receipt(self, update: impl FnOnce(&mut ReceiptInput)) {
        self.update_entry(|input| {
            if let Some(receipt) = &mut input.receipt {
                update(receipt);
                receipt.reviewed = false;
            }
        });
    }
}

/// Release the form even if a native worker stops replying. Dropping a proposal
/// future never writes the ledger; cancellation also stops cooperative inference.
async fn await_model(
    future: crate::UiFuture<Response>,
    operation: &ModelOperation,
    limit: std::time::Duration,
) -> Result<Option<Response>, AppError> {
    let deadline = tokio::time::sleep(limit);
    tokio::pin!(future, deadline);
    loop {
        tokio::select! {
            biased;
            result = &mut future => return if operation.cancelled.load(Ordering::Relaxed) { Ok(None) } else { result.map(Some) },
            _ = &mut deadline => {
                operation.cancelled.store(true, Ordering::Relaxed);
                return Err(AppError::Input("AI ใช้เวลานานเกินไป ลองใหม่หรือใช้แบบฟอร์ม ยังไม่มีรายการถูกบันทึก".into()));
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                if operation.cancelled.load(Ordering::Relaxed) {
                    return Ok(None);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::{UiFuture, UiGateway};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    #[tokio::test]
    async fn stalled_model_times_out_and_cancels_native_work() {
        let operation = ModelOperation::default();
        let result = await_model(
            Box::pin(std::future::pending()),
            &operation,
            Duration::from_millis(20),
        )
        .await;
        assert!(result.is_err());
        assert!(operation.cancelled.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn cancel_does_not_wait_for_a_stalled_worker_or_accept_a_late_proposal() {
        let operation = ModelOperation::default();
        let control = operation.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            control.cancelled.store(true, Ordering::Relaxed);
        });
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            await_model(
                Box::pin(std::future::pending()),
                &operation,
                Duration::from_secs(90),
            ),
        )
        .await
        .expect("cancel must release the UI")
        .expect("cancelled result");
        assert!(result.is_none());
        let late = await_model(
            Box::pin(async { Ok(Response::Resolved(QuickResolution::Summary)) }),
            &operation,
            Duration::from_secs(90),
        )
        .await
        .expect("late result rejected");
        assert!(late.is_none());
    }

    struct DelayedRefresh {
        calls: Arc<AtomicUsize>,
    }
    impl UiGateway for DelayedRefresh {
        fn request(&self, command: Command) -> UiFuture<Response> {
            let calls = Arc::clone(&self.calls);
            Box::pin(async move {
                match command {
                    Command::CreateAccount { .. } => Ok(Response::Committed(CommitOutcome::Saved(
                        "00000000-0000-0000-0000-000000000001".parse()?,
                    ))),
                    Command::Load => {
                        calls.fetch_add(1, Ordering::SeqCst);
                        // The preview/dialog unmounts before the refresh completes.
                        tokio::time::sleep(Duration::from_millis(25)).await;
                        Ok(Response::Dashboard(dashboard(
                            LedgerState::default(),
                            "2026-09-20".parse()?,
                        )?))
                    }
                    _ => Err(AppError::Input("unexpected test command".into())),
                }
            })
        }
        fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
            Box::pin(async { Ok(None) })
        }
        fn scan_receipt(
            &self,
            _: dioxus::html::FileData,
            _: Arc<AtomicBool>,
        ) -> UiFuture<(ReceiptImage, String)> {
            Box::pin(async { Err(AppError::Input("unexpected scan".into())) })
        }
    }

    fn use_test_state() -> UiState {
        let gateway = use_context::<Gateway>();
        UiState {
            gateway: use_signal(|| gateway),
            view: use_signal(|| None),
            page: use_signal(|| Page::Overview),
            busy: use_signal(|| false),
            notice: use_signal(|| None),
            account_form: use_signal(|| true),
            export_form: use_signal(|| false),
            account_deletion: use_signal(|| None),
            input: use_signal(|| None),
            prepared: use_signal(|| None),
            guidance: use_signal(String::new),
            receipt: use_signal(|| None),
            scan_cancel: use_signal(|| None),
            model_operation: use_signal(|| None),
            model_choices: use_signal(|| None),
        }
    }

    #[component]
    fn Harness() -> Element {
        let store = use_test_state();
        use_context_provider(|| store);
        let ready =
            store.view.read().is_some() && !*store.busy.read() && !*store.account_form.read();
        rsx! {
            if *store.account_form.read() { OriginatingChild {} }
            div { if ready { "refreshed-and-ready" } else { "pending" } }
        }
    }

    #[component]
    fn OriginatingChild() -> Element {
        let store = use_context::<UiState>();
        use_effect(move || {
            store.send(Command::CreateAccount {
                name: "cash".into(),
                kind: ledger_domain::AccountKind::Cash,
                opening: "0".into(),
            })
        });
        rsx! { div { "originating-dialog" } }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn commit_refresh_survives_unmounting_the_originating_component() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut dom = VirtualDom::new(Harness);
        dom.insert_any_root_context(Box::new(Gateway(Arc::new(DelayedRefresh {
            calls: Arc::clone(&calls),
        }))));
        dom.rebuild_in_place();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                if dioxus_ssr::render(&dom).contains("refreshed-and-ready") {
                    break;
                }
            }
        })
        .await
        .expect("a committed entry must refresh and release busy after the child closes");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!dioxus_ssr::render(&dom).contains("originating-dialog"));
    }

    #[derive(Clone, Copy)]
    enum ManualStart {
        Idle,
        StalledModel,
        Saving,
    }

    struct ManualGateway {
        requests: Arc<AtomicUsize>,
        models: Arc<AtomicUsize>,
    }
    impl UiGateway for ManualGateway {
        fn request(&self, _: Command) -> UiFuture<Response> {
            self.requests.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { Err(AppError::Input("unexpected request".into())) })
        }
        fn resolve_text(&self, _: String, _: ModelOperation) -> UiFuture<Response> {
            self.models.fetch_add(1, Ordering::SeqCst);
            Box::pin(std::future::pending())
        }
        fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
            Box::pin(async { Ok(None) })
        }
        fn scan_receipt(
            &self,
            _: dioxus::html::FileData,
            _: Arc<AtomicBool>,
        ) -> UiFuture<(ReceiptImage, String)> {
            Box::pin(async { Err(AppError::Input("unexpected scan".into())) })
        }
    }

    #[component]
    fn ManualHarness() -> Element {
        let mut store = use_test_state();
        let start = use_context::<ManualStart>();
        let today = "2026-09-21".parse().expect("fixture date");
        use_effect(move || {
            store.view.set(Some(
                dashboard(LedgerState::default(), today).expect("fixture"),
            ));
            match start {
                ManualStart::Idle => {}
                ManualStart::StalledModel => store.resolve_text("กาแฟ 80".into()),
                ManualStart::Saving => store.busy.set(true),
            }
            store.new_entry();
        });
        let ready = *store.page.read() == Page::Manual
            && store.input.read().as_ref() == Some(&EntryInput::empty(today))
            && !*store.busy.read()
            && store.model_choices.read().is_none()
            && store.prepared.read().is_none();
        let protected = matches!(start, ManualStart::Saving)
            && *store.busy.read()
            && *store.page.read() == Page::Overview
            && store.input.read().is_none();
        rsx! { div { if ready { "manual-ready" } else if protected { "save-protected" } else { "pending" } } }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn manual_entry_bypasses_ai_and_recovers_from_a_stalled_model() {
        for start in [
            ManualStart::Idle,
            ManualStart::StalledModel,
            ManualStart::Saving,
        ] {
            let requests = Arc::new(AtomicUsize::new(0));
            let models = Arc::new(AtomicUsize::new(0));
            let mut dom = VirtualDom::new(ManualHarness);
            dom.insert_any_root_context(Box::new(start));
            dom.insert_any_root_context(Box::new(Gateway(Arc::new(ManualGateway {
                requests: Arc::clone(&requests),
                models: Arc::clone(&models),
            }))));
            dom.rebuild_in_place();
            let expected = if matches!(start, ManualStart::Saving) {
                "save-protected"
            } else {
                "manual-ready"
            };
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    dom.wait_for_work().await;
                    dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
                    if dioxus_ssr::render(&dom).contains(expected) {
                        break;
                    }
                }
            })
            .await
            .expect(
                "manual fallback must open without waiting for AI and must respect an active save",
            );
            assert_eq!(requests.load(Ordering::SeqCst), 0);
            assert_eq!(
                models.load(Ordering::SeqCst),
                usize::from(matches!(start, ManualStart::StalledModel))
            );
        }
    }
}
