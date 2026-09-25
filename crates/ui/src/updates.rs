use crate::{
    components::*,
    i18n::tr,
    state::{UiState, spawn_session},
};
use dioxus::prelude::*;
use ledger_application::*;
use std::sync::atomic::Ordering;

#[derive(Clone, Copy)]
pub struct Updates {
    pub checked: Signal<Option<UpdateCheck>>,
    pub checking: Signal<bool>,
    pub operation: Signal<Option<UpdateOperation>>,
    pub ready: Signal<bool>,
    pub error: Signal<Option<String>>,
    pub progress: Signal<u64>,
}
pub fn use_updates() -> Updates {
    let updates = Updates {
        checked: use_signal(|| None),
        checking: use_signal(|| false),
        operation: use_signal(|| None),
        ready: use_signal(|| false),
        error: use_signal(|| None),
        progress: use_signal(|| 0),
    };
    use_context_provider(|| updates);
    use_drop(move || {
        if let Some(op) = updates.operation.peek().as_ref() {
            op.cancelled.store(true, Ordering::Relaxed);
        }
    });
    updates
}
impl Updates {
    pub fn check(mut self, store: UiState) {
        if *self.checking.peek() || self.operation.peek().is_some() || *store.busy.peek() {
            return;
        }
        self.checking.set(true);
        self.error.set(None);
        let gateway = store.gateway.peek().clone();
        spawn_session(async move {
            match gateway.0.check_update().await {
                Ok(result) => {
                    if self.checked.peek().as_ref() != Some(&result) {
                        self.ready.set(false);
                    }
                    self.checked.set(Some(result));
                }
                Err(error) => self.error.set(Some(error.to_string())),
            }
            self.checking.set(false);
        });
    }
    fn download(mut self, mut store: UiState, release: AppRelease) {
        if self.operation.peek().is_some() || *store.busy.peek() {
            return;
        }
        let operation = UpdateOperation::default();
        self.operation.set(Some(operation.clone()));
        self.progress.set(0);
        self.error.set(None);
        self.ready.set(false);
        store.busy.set(true);
        let gateway = store.gateway.peek().clone();
        spawn_session(async move {
            let future = gateway.0.download_update(release, operation.clone());
            tokio::pin!(future);
            let result = loop {
                tokio::select! {
                    result = &mut future => break result,
                    _ = tokio::time::sleep(std::time::Duration::from_millis(200)) => self.progress.set(operation.downloaded.load(Ordering::Relaxed)),
                }
            };
            match result {
                Ok(()) => self.ready.set(true),
                Err(error) => self.error.set(Some(error.to_string())),
            }
            self.operation.set(None);
            store.busy.set(false);
        });
    }
}

#[component]
pub fn UpdateSettings() -> Element {
    let mut store = use_context::<UiState>();
    let mut updates = use_context::<Updates>();
    let supported = store.gateway.read().0.supports_updates();
    let available = updates
        .checked
        .read()
        .as_ref()
        .and_then(|c| c.available.clone());
    let working = *store.busy.read() || (updates.checking)();
    rsx! {
        h2 { class: "preferences-section-title", {tr("อัปเดตแอป")} }
        section { class: "settings-group",
            div { class: "setting-row", div { class: "setting-copy", h3 { "Ledgers Bro" } p { {tr("รุ่นปัจจุบัน")} ": " {updates.checked.read().as_ref().map(|c| c.current.clone()).unwrap_or_else(|| env!("CARGO_PKG_VERSION").into())} } }
                if supported { button { class: "soft-button", disabled: working, onclick: move |_| updates.check(store), Icon { name: "refresh", size: 18 } {tr(if (updates.checking)() { "กำลังตรวจอัปเดต…" } else { "ตรวจอัปเดต" })} } }
            }
            p { class: "field-hint", {tr("ดาวน์โหลดตัวติดตั้งที่ตรงกับอุปกรณ์ ตรวจไฟล์ก่อนติดตั้ง และเก็บข้อมูลเดิมเมื่ออัปเดตทับแอปเดิม")} }
            a { href: RELEASE_PAGE, target: "_blank", rel: "noopener noreferrer", {tr("เปิด Release ล่าสุด")} }
            if let Some(release) = available {
                div { class: "update-available",
                    h3 { {tr("มีเวอร์ชันใหม่")} ": {release.version}" }
                    p { "{release.filename}" }
                    p { {tr("ขนาดดาวน์โหลด")} ": " {format!("{:.1} MB", release.bytes as f64 / 1048576.0)} }
                    if let Some(operation) = updates.operation.read().clone() {
                        progress { max: "{release.bytes}", value: "{updates.progress}", "aria-label": tr("กำลังดาวน์โหลดอัปเดต") }
                        p { {format!("{:.0}%", *updates.progress.read() as f64 / release.bytes as f64 * 100.0)} }
                        button { class: "soft-button", onclick: move |_| operation.cancelled.store(true, Ordering::Relaxed), {tr("ยกเลิก")} }
                    } else if (updates.ready)() {
                        p { role: "status", {tr("ดาวน์โหลดและตรวจไฟล์แล้ว พร้อมติดตั้ง")} }
                        button { class: "primary", disabled: working, onclick: move |_| {
                            let gateway = store.gateway.peek().clone();
                            crate::confirmation::ask(tr("ติดตั้งอัปเดตตอนนี้? Windows จะปิดแอปและเปิดตัวติดตั้ง MSI ส่วน Android จะเปิดหน้าติดตั้งของระบบ กรุณายืนยันในหน้าต่างระบบอีกครั้ง"), move |_| {
                                if *store.busy.peek() { return; }
                                store.busy.set(true); let gateway = gateway.clone(); updates.error.set(None);
                                spawn_session(async move {
                                    match gateway.0.install_update().await {
                                        Ok(UpdateInstall::InstallerOpened) => { store.notice.set(Some((false, "เปิดตัวติดตั้งแล้ว กรุณาทำตามขั้นตอนของระบบ".into()))); gateway.0.close_after_update(); }
                                        Ok(UpdateInstall::PermissionRequired) => updates.error.set(Some("อนุญาตให้แอปนี้ติดตั้งอัปเดตในหน้าตั้งค่าระบบ แล้วกลับมากดติดตั้งอีกครั้ง".into())),
                                        Err(error) => updates.error.set(Some(error.to_string())),
                                    }
                                    store.busy.set(false);
                                });
                            });
                        }, {tr("ติดตั้งอัปเดต")} }
                    } else { button { class: "primary", disabled: working, onclick: move |_| updates.download(store, release.clone()), Icon { name: "download", size: 18 } {tr("ดาวน์โหลดอัปเดต")} } }
                }
            } else if updates.checked.read().is_some() && updates.error.read().is_none() { p { role: "status", {tr("กำลังใช้เวอร์ชันล่าสุด")} } }
            if let Some(error) = (updates.error)() { p { class: "form-error", role: "alert", "{tr(&error)}" } }
        }
    }
}
