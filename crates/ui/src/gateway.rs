use crate::voice::{VoiceEvent, VoiceSessionId, voice_unavailable};
use ledger_application::{
    AppError, Command, CsvExport, ModelAvailability, ModelOperation, ReceiptImage, Response,
};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, atomic::AtomicBool},
};

pub type ReceiptScan = (String, Result<(ReceiptImage, String), AppError>);

pub type UiFuture<T> = Pin<Box<dyn Future<Output = Result<T, AppError>> + Send>>;
pub trait UiGateway: Send + Sync {
    fn supports_updates(&self) -> bool {
        false
    }
    fn check_update(&self) -> UiFuture<ledger_application::UpdateCheck> {
        Box::pin(async { Err(AppError::WorkerStopped) })
    }
    fn download_update(
        &self,
        _release: ledger_application::AppRelease,
        _operation: ledger_application::UpdateOperation,
    ) -> UiFuture<()> {
        Box::pin(async { Err(AppError::WorkerStopped) })
    }
    fn install_update(&self) -> UiFuture<ledger_application::UpdateInstall> {
        Box::pin(async { Err(AppError::WorkerStopped) })
    }
    fn close_after_update(&self) {}
    fn alert_preferences(&self) -> UiFuture<ledger_application::AlertPreferences> {
        Box::pin(async { Ok(Default::default()) })
    }
    fn save_alert_preferences(&self, _prefs: ledger_application::AlertPreferences) -> UiFuture<()> {
        Box::pin(async { Ok(()) })
    }
    fn enable_system_notifications(&self) -> UiFuture<bool> {
        Box::pin(async { Ok(false) })
    }
    fn system_notification(&self, _title: String, _body: String) -> UiFuture<()> {
        Box::pin(async { Ok(()) })
    }
    fn profiles(
        &self,
        _command: ledger_application::ProfileCommand,
    ) -> UiFuture<ledger_application::ProfileResponse> {
        Box::pin(async { Err(ledger_application::login_required()) })
    }
    fn authenticated(&self, _token: &str) -> Result<Gateway, AppError> {
        Err(ledger_application::login_required())
    }
    fn crypto_prices(&self) -> UiFuture<ledger_domain::CryptoPrices> {
        Box::pin(async { Err(ledger_domain::DomainError::InvalidCryptoPrice.into()) })
    }
    fn model_availability(&self) -> UiFuture<ModelAvailability> {
        Box::pin(async { Ok(ModelAvailability::Missing) })
    }
    fn delete_model(&self, _id: ledger_application::LocalModelId) -> UiFuture<()> {
        Box::pin(async { Err(AppError::WorkerStopped) })
    }
    fn install_model(&self, _operation: ModelOperation) -> UiFuture<()> {
        Box::pin(async {
            Err(AppError::Input("รุ่นนี้ยังติดตั้ง AI ไม่ได้".into()))
        })
    }
    fn model_settings(&self) -> UiFuture<ledger_application::ModelSettingsSnapshot> {
        Box::pin(async {
            Err(AppError::Input("การเลือกโมเดลยังไม่พร้อมบนอุปกรณ์นี้".into()))
        })
    }
    fn activate_model(
        &self,
        _id: ledger_application::LocalModelId,
        _operation: ModelOperation,
    ) -> UiFuture<()> {
        Box::pin(async {
            Err(AppError::Input("การเลือกโมเดลยังไม่พร้อมบนอุปกรณ์นี้".into()))
        })
    }
    fn resolve_text(&self, text: String, _operation: ModelOperation) -> UiFuture<Response> {
        self.request(Command::Resolve(text))
    }
    /// Native host availability, not a promise that a Thai model is installed.
    fn supports_voice(&self) -> bool {
        false
    }
    fn begin_voice(&self, _id: VoiceSessionId) -> UiFuture<()> {
        voice_unavailable()
    }
    fn begin_voice_model_download(&self, _id: VoiceSessionId) -> UiFuture<()> {
        voice_unavailable()
    }
    /// Adapters pace polling and enforce bounded setup/listening/result deadlines.
    fn poll_voice(&self, _id: VoiceSessionId) -> UiFuture<VoiceEvent> {
        voice_unavailable()
    }
    fn stop_voice(&self, _id: VoiceSessionId) {}
    /// Fire-and-forget cleanup, safe to call from a dropped UI task.
    fn cancel_voice(&self, _id: VoiceSessionId) {}
    fn request(&self, command: Command) -> UiFuture<Response>;
    /// The platform owns the save dialog and filesystem, not the component.
    fn save_csv(&self, csv: CsvExport) -> UiFuture<Option<String>>;
    fn save_document(
        &self,
        _document: ledger_application::DocumentExport,
    ) -> UiFuture<Option<String>> {
        Box::pin(async {
            Err(AppError::Input("เครื่องนี้ยังส่งออกไฟล์นี้ไม่ได้".into()))
        })
    }
    fn pick_document(
        &self,
        _kind: ledger_application::ImportFileKind,
    ) -> UiFuture<Option<Arc<[u8]>>> {
        Box::pin(async {
            Err(AppError::Input("เครื่องนี้ยังนำเข้าไฟล์ไม่ได้".into()))
        })
    }
    fn scan_receipt(
        &self,
        file: dioxus::html::FileData,
        cancel: Arc<AtomicBool>,
    ) -> UiFuture<(ReceiptImage, String)>;
    fn uses_native_receipt_picker(&self) -> bool {
        false
    }
    fn pick_receipts(&self, _cancel: Arc<AtomicBool>) -> UiFuture<Vec<ReceiptScan>> {
        Box::pin(async {
            Err(AppError::Input("เครื่องนี้ยังเลือกภาพแบบนี้ไม่ได้".into()))
        })
    }
}
#[derive(Clone)]
pub struct Gateway(pub Arc<dyn UiGateway>);
impl PartialEq for Gateway {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
#[derive(Clone)]
pub struct HostInfo {
    pub art: Arc<ArtAssets>,
    pub isolated: bool,
    pub receipt_ocr_available: bool,
    pub preview_label: &'static str,
}

pub struct ArtAssets {
    pub hero: String,
    pub phone: String,
    pub accounts: String,
    pub history: String,
    pub calendar: String,
    pub tax: String,
    pub off_duty: String,
}

impl ArtAssets {
    /// Presentation assets are bundled once and shared by every native host.
    pub fn bundled() -> Self {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let encode = |bytes: &[u8]| format!("data:image/png;base64,{}", STANDARD.encode(bytes));
        let hero = encode(include_bytes!("../assets/uncle-crab/hero.png"));
        Self {
            accounts: hero.clone(),
            hero,
            phone: encode(include_bytes!("../assets/uncle-crab/phone.png")),
            history: encode(include_bytes!("../assets/uncle-crab/history.png")),
            calendar: encode(include_bytes!("../assets/uncle-crab/calendar.png")),
            tax: encode(include_bytes!("../assets/uncle-crab/tax.png")),
            off_duty: encode(include_bytes!("../assets/uncle-crab/off-duty.png")),
        }
    }

    pub fn for_page(&self, page: crate::state::Page) -> &str {
        use crate::state::Page;
        match page {
            Page::Overview | Page::Accounts => &self.hero,
            Page::Chat | Page::Receivables => &self.phone,
            Page::Manual | Page::Transactions => &self.history,
            Page::Recurring => &self.calendar,
            Page::Tax => &self.tax,
            Page::Settings => &self.off_duty,
        }
    }
}
