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
    worker: LedgerWorker,
    model: ModelWorker,
    ocr: TesseractOcr,
}
impl UiGateway for DesktopGateway {
    fn model_availability(&self) -> UiFuture<ModelAvailability> {
        let model = self.model.clone();
        Box::pin(async move { model.availability().await })
    }
    fn install_model(&self, operation: ModelOperation) -> UiFuture<()> {
        let model = self.model.clone();
        Box::pin(async move { model.install(operation).await })
    }
    fn resolve_text(&self, text: String, operation: ModelOperation) -> UiFuture<Response> {
        let model = self.model.clone();
        let worker = self.worker.clone();
        Box::pin(async move { model.resolve(&worker, text, operation).await })
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
        Box::pin(async move { worker.request(command).await })
    }
    fn save_csv(&self, csv: CsvExport) -> UiFuture<Option<String>> {
        Box::pin(async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title("ส่งออกรายงานบัญชีคู่เป็น CSV")
                .set_file_name(&csv.filename)
                .add_filter("CSV", &["csv"])
                .save_file()
                .await;
            let Some(file) = file else {
                return Ok(None);
            };
            file.write(csv.contents.as_bytes())
                .await
                .map_err(|_| AppError::Input("เขียนไฟล์ส่งออกไม่ได้ กรุณาลองเลือกที่บันทึกใหม่".into()))?;
            Ok(Some(file.file_name()))
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
    let worker = LedgerWorker::start(directory.join("ledger.sqlite3"))?;
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
        worker,
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
    let mut config = Config::new()
        .with_window(
            WindowBuilder::new()
                .with_title("Ledgers Bro")
                .with_inner_size(if mobile_preview {
                    LogicalSize::new(430.0, 860.0)
                } else {
                    LogicalSize::new(1280.0, 950.0)
                })
                .with_min_inner_size(LogicalSize::new(360.0, 640.0)),
        )
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
        .launch(ledger_ui::App);
    Ok(())
}
