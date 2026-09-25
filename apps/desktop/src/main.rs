#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::{
    desktop::{Config, LogicalSize, WindowBuilder},
    prelude::*,
};
use ledger_application::{AppError, Command, CsvExport, MAX_RECEIPT_BYTES, ReceiptImage, Response};
use ledger_application::{ModelAvailability, ModelOperation};
use ledger_infrastructure::{LedgerWorker, ModelWorker, TesseractOcr};
use ledger_ui::{ArtAssets, Gateway, HostInfo, UiFuture, UiGateway};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

struct DesktopGateway {
    worker: Option<LedgerWorker>,
    profiles: ledger_infrastructure::ProfileWorker,
    model: ModelWorker,
    ocr: TesseractOcr,
}
impl UiGateway for DesktopGateway {
    fn profiles(
        &self,
        command: ledger_application::ProfileCommand,
    ) -> UiFuture<ledger_application::ProfileResponse> {
        let worker = self.profiles.clone();
        Box::pin(async move { worker.request(command).await })
    }
    fn authenticated(&self, token: &str) -> Result<Gateway, AppError> {
        Ok(Gateway(Arc::new(Self {
            worker: Some(self.profiles.ledger(token)?),
            profiles: self.profiles.clone(),
            model: self.model.clone(),
            ocr: self.ocr.clone(),
        })))
    }
    fn crypto_prices(&self) -> UiFuture<ledger_domain::CryptoPrices> {
        Box::pin(ledger_infrastructure::fetch_crypto_prices())
    }
    fn model_availability(&self) -> UiFuture<ModelAvailability> {
        let model = self.model.clone();
        Box::pin(async move { model.availability().await })
    }
    fn install_model(&self, operation: ModelOperation) -> UiFuture<()> {
        let model = self.model.clone();
        Box::pin(async move { model.install(operation).await })
    }
    fn model_settings(&self) -> UiFuture<ledger_application::ModelSettingsSnapshot> {
        let model = self.model.clone();
        Box::pin(async move { model.settings().await })
    }
    fn activate_model(
        &self,
        id: ledger_application::LocalModelId,
        operation: ModelOperation,
    ) -> UiFuture<()> {
        let model = self.model.clone();
        Box::pin(async move { model.activate(id, operation).await })
    }

    fn resolve_text(&self, text: String, operation: ModelOperation) -> UiFuture<Response> {
        let model = self.model.clone();
        let worker = self.worker.clone();
        Box::pin(async move {
            model
                .resolve(
                    &worker.ok_or_else(ledger_application::login_required)?,
                    text,
                    operation,
                )
                .await
        })
    }
    fn scan_receipt(
        &self,
        file: dioxus::html::FileData,
        cancel: Arc<AtomicBool>,
    ) -> UiFuture<(ReceiptImage, String)> {
        scan_receipt_path(
            self.ocr.clone(),
            file.path(),
            cancel,
            Arc::new(std::sync::atomic::AtomicU64::new(0)),
        )
    }
    fn uses_native_receipt_picker(&self) -> bool {
        true
    }
    fn pick_receipts(&self, cancel: Arc<AtomicBool>) -> UiFuture<Vec<ledger_ui::ReceiptScan>> {
        let ocr = self.ocr.clone();
        Box::pin(async move {
            let Some(files) = rfd::AsyncFileDialog::new()
                .set_title("เลือกใบเสร็จได้หลายรูป")
                .add_filter("Receipt images", &["jpg", "jpeg", "png", "heic", "heif"])
                .pick_files()
                .await
            else {
                return Ok(Vec::new());
            };
            if files.len() > ledger_application::MAX_RECEIPT_IMAGES {
                return Err(AppError::Input("เลือกได้ครั้งละไม่เกิน 8 รูป".into()));
            }
            let budget = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let mut results = Vec::new();
            let mut total = 0u64;
            for file in files {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                let result = scan_receipt_path(
                    ocr.clone(),
                    file.path().to_owned(),
                    cancel.clone(),
                    budget.clone(),
                )
                .await;
                if let Ok((image, _)) = &result {
                    total += image.bytes().len() as u64;
                }
                if total > ledger_application::MAX_RECEIPT_BATCH_BYTES {
                    return Err(AppError::Input("รูปใบเสร็จรวมต้องไม่เกิน 128 MB".into()));
                }
                results.push((file.file_name(), result));
            }
            Ok(results)
        })
    }
    fn request(&self, command: Command) -> UiFuture<Response> {
        let worker = self.worker.clone();
        Box::pin(async move {
            worker
                .ok_or_else(ledger_application::login_required)?
                .request(command)
                .await
        })
    }
    fn save_csv(&self, csv: CsvExport) -> UiFuture<Option<String>> {
        self.save_document(csv.into())
    }
    fn save_document(
        &self,
        document: ledger_application::DocumentExport,
    ) -> UiFuture<Option<String>> {
        Box::pin(async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title("เลือกที่บันทึกไฟล์")
                .set_file_name(&document.filename)
                .add_filter(
                    "Ledgers Bro",
                    &[if document.mime == "text/csv" {
                        "csv"
                    } else {
                        "lbro"
                    }],
                )
                .save_file()
                .await;
            let Some(file) = file else {
                return Ok(None);
            };
            file.write(&document.bytes)
                .await
                .map_err(|_| AppError::Input("เขียนไฟล์ส่งออกไม่ได้ กรุณาลองเลือกที่บันทึกใหม่".into()))?;
            Ok(Some(file.file_name()))
        })
    }
    fn pick_document(
        &self,
        kind: ledger_application::ImportFileKind,
    ) -> UiFuture<Option<Arc<[u8]>>> {
        Box::pin(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("เลือกไฟล์นำเข้า")
                .add_filter(
                    "Ledgers Bro",
                    &[if kind == ledger_application::ImportFileKind::Csv {
                        "csv"
                    } else {
                        "lbro"
                    }],
                )
                .pick_file()
                .await
            else {
                return Ok(None);
            };
            let path = file.path().to_owned();
            let (send, receive) = futures_channel::oneshot::channel();
            std::thread::Builder::new()
                .name("document-import".into())
                .spawn(move || {
                    let result = (|| {
                        let input = std::fs::File::open(path)
                            .map_err(|_| AppError::Input("เปิดไฟล์ไม่ได้".into()))?;
                        let mut bytes = Vec::new();
                        input
                            .take(kind.max_bytes() as u64 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|_| AppError::Input("อ่านไฟล์ไม่ได้".into()))?;
                        if bytes.len() > kind.max_bytes() {
                            return Err(AppError::Input("ไฟล์ใหญ่เกินขนาดที่รองรับ".into()));
                        }
                        Ok(Some(Arc::from(bytes)))
                    })();
                    let _ = send.send(result);
                })
                .map_err(|_| AppError::WorkerStopped)?;
            receive.await.map_err(|_| AppError::WorkerStopped)?
        })
    }
}

fn scan_receipt_path(
    ocr: TesseractOcr,
    path: PathBuf,
    cancel: Arc<AtomicBool>,
    budget: Arc<std::sync::atomic::AtomicU64>,
) -> UiFuture<(ReceiptImage, String)> {
    let (tx, rx) = futures_channel::oneshot::channel();
    let spawned = std::thread::Builder::new()
        .name("receipt-ocr".into())
        .spawn(move || {
            let result = (|| {
                let input = std::fs::File::open(path)
                    .map_err(|_| AppError::Input("เปิดรูปใบเสร็จไม่ได้".into()))?;
                let mut bytes = Vec::new();
                input
                    .take((MAX_RECEIPT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .map_err(|_| AppError::Input("อ่านรูปใบเสร็จไม่ได้".into()))?;
                let consumed =
                    budget.fetch_add(bytes.len() as u64, std::sync::atomic::Ordering::Relaxed);
                if consumed.saturating_add(bytes.len() as u64)
                    > ledger_application::MAX_RECEIPT_BATCH_BYTES
                {
                    return Err(AppError::Input("รูปใบเสร็จรวมต้องไม่เกิน 128 MB".into()));
                }
                ocr.scan(&ReceiptImage::new(bytes)?, &cancel)
            })();
            let _ = tx.send(result);
        });
    Box::pin(async move {
        spawned.map_err(|_| AppError::Input("เริ่มอ่านใบเสร็จไม่ได้".into()))?;
        rx.await
            .map_err(|_| AppError::Input("ตัวอ่านใบเสร็จหยุดทำงาน กรุณาลองใหม่".into()))?
    })
}

fn main() {
    if let Err(error) = launch() {
        eprintln!("Ledgers Bro: {error}");
        let _ = rfd::MessageDialog::new()
            .set_title("เปิด Ledgers Bro ไม่สำเร็จ")
            .set_description(error.to_string())
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}

fn launch() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mut directory = None;
    let mut inspect_port = None;
    let mut ocr_directory = None;
    let mut model_directory = None;
    let mut mobile_preview = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--mobile-preview" => mobile_preview = true,
            "--model-dir" => {
                model_directory = Some(PathBuf::from(
                    args.next().ok_or("--model-dir requires a directory")?,
                ));
            }
            "--ocr-dir" => {
                ocr_directory = Some(PathBuf::from(
                    args.next().ok_or("--ocr-dir requires a directory")?,
                ))
            }
            "--data-dir" => {
                directory = Some(PathBuf::from(
                    args.next().ok_or("--data-dir requires a directory")?,
                ))
            }
            "--inspect-port" if cfg!(debug_assertions) => {
                inspect_port = Some(
                    args.next()
                        .ok_or("--inspect-port requires a port")?
                        .parse::<u16>()?,
                )
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }
    let isolated = directory.is_some();
    let directory = match directory {
        Some(path) => path,
        None => directories::ProjectDirs::from("com", "Dancing With My Code", "Ledgers Bro")
            .ok_or("cannot determine the application data directory")?
            .data_local_dir()
            .to_owned(),
    };
    std::fs::create_dir_all(&directory)?;
    let profiles = ledger_infrastructure::ProfileWorker::start(&directory)?;
    let ocr_directory = ocr_directory.unwrap_or_else(|| {
        if cfg!(debug_assertions) {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tools/ocr")
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("ocr")))
                .unwrap_or_default()
        }
    });
    let gateway = Gateway(Arc::new(DesktopGateway {
        worker: None,
        profiles,
        model: ModelWorker::start(model_directory.unwrap_or_else(|| directory.join("models")))?,
        ocr: TesseractOcr::new(ocr_directory),
    }));
    let art = Arc::new(ArtAssets::bundled());
    let host = HostInfo {
        art,
        isolated,
        receipt_ocr_available: true,
        preview_label: "สมุดบัญชีแยกสำหรับทดลอง • Desktop preview",
    };
    let window = WindowBuilder::new()
        .with_title("Ledgers Bro")
        .with_inner_size(if mobile_preview {
            LogicalSize::new(430.0, 860.0)
        } else {
            LogicalSize::new(1280.0, 950.0)
        })
        .with_min_inner_size(LogicalSize::new(360.0, 640.0));
    // Standalone Dioxus debug runs share a temporary window-position cache.
    // Windows may save a minimized window at (-32000, -32000); an explicit
    // starting position prevents restoring the next test run outside the screen.
    #[cfg(all(target_os = "windows", debug_assertions))]
    let window = window.with_position(dioxus::desktop::LogicalPosition::new(40.0, 40.0));
    let mut config = Config::new()
        .with_window(window)
        .with_data_directory(directory.join(match inspect_port {
            Some(port) => format!("webview-inspect-{port}"),
            None => "webview".into(),
        }))
        .with_background_color((255, 227, 165, 255));
    // macOS routes Command-C/V/A through the native Edit menu, including WKWebView
    // text fields. Removing that menu makes pasted quick-entry prompts disappear.
    #[cfg(not(target_os = "macos"))]
    {
        config = config.with_menu(None);
    }
    if let Some(port) = inspect_port {
        config = config.with_windows_browser_args(format!("--remote-debugging-port={port}"));
    }
    LaunchBuilder::desktop()
        .with_cfg(config)
        .with_context(gateway)
        .with_context(host)
        .launch(ledger_ui::LoginRoot);
    Ok(())
}
