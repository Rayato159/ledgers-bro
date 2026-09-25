#![allow(clippy::expect_used)]
use super::*;
use crate::{Gateway, UiFuture, UiGateway};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicUsize},
    },
    time::Duration,
};

#[derive(Default)]
struct DownloadProbe {
    calls: AtomicUsize,
    checks: AtomicUsize,
    finish: AtomicU8,
    installed: AtomicBool,
    operation: Mutex<Option<ModelOperation>>,
}
impl UiGateway for DownloadProbe {
    fn request(&self, _: Command) -> UiFuture<Response> {
        Box::pin(async { Err(AppError::Input("No financial commands in this test".into())) })
    }
    fn save_csv(&self, _: CsvExport) -> UiFuture<Option<String>> {
        Box::pin(async { Ok(None) })
    }
    fn scan_receipt(
        &self,
        _: dioxus::html::FileData,
        _: Arc<AtomicBool>,
    ) -> UiFuture<(ReceiptImage, String)> {
        Box::pin(async { Err(AppError::WorkerStopped) })
    }
    fn model_settings(&self) -> UiFuture<ModelSettingsSnapshot> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        let installed = self.installed.load(Ordering::SeqCst);
        Box::pin(async move {
            Ok(ModelSettingsSnapshot {
                selected: if installed {
                    LocalModelId::Balanced
                } else {
                    LocalModelId::Small
                },
                downloaded: if installed {
                    vec![LocalModelId::Balanced]
                } else {
                    vec![]
                },
                device: ModelDevice {
                    total_memory: Some(32 << 30),
                    available_memory: Some(24 << 30),
                    free_disk: Some(64 << 30),
                    cpu_threads: 8,
                    mobile: false,
                },
            })
        })
    }
}
// A cloneable adapter lets the fake worker complete while its originating UI is gone.
struct DownloadGateway(Arc<DownloadProbe>);
impl UiGateway for DownloadGateway {
    fn request(&self, command: Command) -> UiFuture<Response> {
        self.0.request(command)
    }
    fn save_csv(&self, csv: CsvExport) -> UiFuture<Option<String>> {
        self.0.save_csv(csv)
    }
    fn scan_receipt(
        &self,
        file: dioxus::html::FileData,
        cancel: Arc<AtomicBool>,
    ) -> UiFuture<(ReceiptImage, String)> {
        self.0.scan_receipt(file, cancel)
    }
    fn model_settings(&self) -> UiFuture<ModelSettingsSnapshot> {
        self.0.model_settings()
    }
    fn activate_model(&self, id: LocalModelId, operation: ModelOperation) -> UiFuture<()> {
        assert_eq!(id, LocalModelId::Balanced);
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        *self.0.operation.lock().expect("probe") = Some(operation.clone());
        let probe = self.0.clone();
        Box::pin(async move {
            operation
                .downloaded_bytes
                .store(1_234_567, Ordering::Relaxed);
            loop {
                if operation.cancelled.load(Ordering::Relaxed) {
                    return Err(AppError::Input("Cancelled by test user".into()));
                }
                match probe.finish.load(Ordering::SeqCst) {
                    1 => {
                        probe.installed.store(true, Ordering::SeqCst);
                        return Ok(());
                    }
                    2 => return Err(AppError::Input("Simulated connection failure".into())),
                    _ => tokio::time::sleep(Duration::from_millis(1)).await,
                }
            }
        })
    }
}

#[derive(Clone, Copy)]
struct Handles {
    scope: ScopeId,
    library: ModelLibrary,
    store: UiState,
    visible: Signal<bool>,
}
type Capture = Rc<RefCell<Option<Handles>>>;
fn harness() -> Element {
    let store = crate::state::tests::use_test_state();
    use_context_provider(|| store);
    let library = use_model_library();
    let visible = use_signal(|| true);
    let capturing = use_signal(|| false);
    let capture = use_context::<Capture>();
    use_hook(move || {
        *capture.borrow_mut() = Some(Handles {
            scope: dioxus::dioxus_core::current_scope_id(),
            library,
            store,
            visible,
        })
    });
    rsx! { div { "data-busy": (store.busy)() } if visible() { ModelSettings { capturing } } }
}
async fn settle(dom: &mut VirtualDom, condition: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
            if condition() {
                break;
            }
            dom.wait_for_work().await;
        }
    })
    .await
    .expect("UI settles");
}
async fn setup() -> (VirtualDom, Handles, Arc<DownloadProbe>) {
    let probe = Arc::new(DownloadProbe::default());
    let capture = Capture::default();
    let mut dom = VirtualDom::new(harness);
    dom.insert_any_root_context(Box::new(capture.clone()));
    dom.insert_any_root_context(Box::new(Gateway(Arc::new(DownloadGateway(probe.clone())))));
    dom.rebuild_in_place();
    let handles = capture.borrow().expect("root handles");
    settle(&mut dom, || handles.library.snapshot.peek().is_some()).await;
    dom.in_scope(handles.scope, || {
        handles
            .library
            .activate(handles.store, LocalModelId::Balanced)
    });
    settle(&mut dom, || probe.calls.load(Ordering::SeqCst) == 1).await;
    (dom, handles, probe)
}
fn show(dom: &mut VirtualDom, mut handles: Handles, visible: bool) {
    dom.in_scope(handles.scope, || handles.visible.set(visible));
    dom.render_immediate(&mut dioxus::dioxus_core::NoOpMutations);
}

#[tokio::test(flavor = "current_thread")]
async fn download_survives_tab_changes_and_completes_while_settings_is_unmounted() {
    let (mut dom, handles, probe) = setup().await;
    let operation = probe
        .operation
        .lock()
        .expect("probe")
        .clone()
        .expect("active");
    show(&mut dom, handles, false);
    assert!(!operation.cancelled.load(Ordering::Relaxed));
    assert!(*handles.store.busy.peek());
    show(&mut dom, handles, true);
    let html = dioxus_ssr::render(&dom);
    assert!(html.contains("value=\"1234567\""), "{html}");
    assert!(
        html.contains("max=\"2497280256\""),
        "4B progress total: {html}"
    );
    assert_eq!(probe.calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        probe.checks.load(Ordering::SeqCst),
        1,
        "no query queued behind download on remount"
    );
    show(&mut dom, handles, false);
    probe.finish.store(1, Ordering::SeqCst);
    settle(&mut dom, || !*handles.store.busy.peek()).await;
    assert!(handles.library.operation.peek().is_none());
    let snapshot = handles.library.snapshot.peek().clone().expect("snapshot");
    assert_eq!(snapshot.selected, LocalModelId::Balanced);
    assert_eq!(snapshot.downloaded, vec![LocalModelId::Balanced]);
    show(&mut dom, handles, true);
    let html = dioxus_ssr::render(&dom);
    assert!(!html.contains("model-download-progress"));
    assert!(html.contains(&*handles.library.message.peek()));
}

#[tokio::test(flavor = "current_thread")]
async fn errors_while_away_survive_remount_without_claiming_install_success() {
    let (mut dom, handles, probe) = setup().await;
    show(&mut dom, handles, false);
    probe.finish.store(2, Ordering::SeqCst);
    settle(&mut dom, || !*handles.store.busy.peek()).await;
    show(&mut dom, handles, true);
    assert!(dioxus_ssr::render(&dom).contains("Simulated connection failure"));
    assert!(!probe.installed.load(Ordering::SeqCst));
    assert!(
        handles
            .library
            .snapshot
            .peek()
            .as_ref()
            .expect("snapshot")
            .downloaded
            .is_empty()
    );
}

#[tokio::test(flavor = "current_thread")]
async fn explicit_cancel_and_session_drop_still_stop_download() {
    let (mut dom, handles, probe) = setup().await;
    dom.in_scope(handles.scope, || handles.library.cancel());
    settle(&mut dom, || !*handles.store.busy.peek()).await;
    assert!(!probe.installed.load(Ordering::SeqCst));
    assert!(dioxus_ssr::render(&dom).contains("Cancelled by test user"));
    drop(dom);
    let (dom, _, probe) = setup().await;
    let operation = probe
        .operation
        .lock()
        .expect("probe")
        .clone()
        .expect("active");
    drop(dom);
    assert!(operation.cancelled.load(Ordering::Relaxed));
}
