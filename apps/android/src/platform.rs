//! Safe JNI adapter: Wry owns the JVM/activity lifetime and dispatches on its UI thread.
use dioxus::mobile::wry;
use futures_channel::oneshot;
use jni::{JNIEnv, objects::JObject};
use ledger_application::{AppError, Command, CsvExport, ReceiptImage, Response};
use ledger_application::{ModelAvailability, ModelOperation};
use ledger_infrastructure::{LedgerWorker, ModelWorker};
use ledger_ui::{Gateway, UiFuture, UiGateway, VoiceEvent, VoiceSessionId};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

struct AndroidGateway {
    worker: LedgerWorker,
    model: ModelWorker,
}

pub(crate) async fn on_activity<T, F>(operation: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> jni::errors::Result<T> + Send + 'static,
{
    let (send, receive) = oneshot::channel();
    wry::prelude::dispatch(move |env, activity, _| {
        let result = operation(env, activity).map_err(|_| {
            // JNI failures must not leave a pending exception on Android's UI thread.
            let _ = env.exception_clear();
            AppError::Input("ติดต่อระบบ Android ไม่สำเร็จ กรุณาลองอีกครั้ง".into())
        });
        let _ = send.send(result);
    });
    receive.await.map_err(|_| AppError::WorkerStopped)?
}

pub async fn initialize() -> Result<Gateway, AppError> {
    let directory: String = on_activity(|env, activity| {
        // Excluded from Android Auto Backup; never place the ledger in shared storage.
        let file = env
            .call_method(activity, "getNoBackupFilesDir", "()Ljava/io/File;", &[])?
            .l()?;
        let path = env
            .call_method(file, "getAbsolutePath", "()Ljava/lang/String;", &[])?
            .l()?;
        Ok(env.get_string(&path.into())?.into())
    })
    .await?;
    let (send, receive) = oneshot::channel();
    std::thread::Builder::new()
        .name("ledger-bootstrap".into())
        .spawn(move || {
            let result = (|| {
                let directory = PathBuf::from(directory);
                std::fs::create_dir_all(&directory)
                    .map_err(|_| AppError::Input("เปิดที่เก็บข้อมูลในเครื่องไม่ได้".into()))?;
                Ok(Gateway(Arc::new(AndroidGateway {
                    worker: LedgerWorker::start(directory.join("ledger.sqlite3"))?,
                    model: ModelWorker::start(directory.join("models"))?,
                })))
            })();
            let _ = send.send(result);
        })
        .map_err(|_| AppError::WorkerStopped)?;
    receive.await.map_err(|_| AppError::WorkerStopped)?
}

impl UiGateway for AndroidGateway {
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
    fn supports_voice(&self) -> bool {
        true
    }
    fn begin_voice(&self, id: VoiceSessionId) -> UiFuture<()> {
        crate::voice::begin(id, "beginVoice")
    }
    fn begin_voice_model_download(&self, id: VoiceSessionId) -> UiFuture<()> {
        crate::voice::begin(id, "beginVoiceModelDownload")
    }
    fn poll_voice(&self, id: VoiceSessionId) -> UiFuture<VoiceEvent> {
        crate::voice::poll(id)
    }
    fn stop_voice(&self, id: VoiceSessionId) {
        crate::voice::control(id, "stopVoice");
    }
    fn cancel_voice(&self, id: VoiceSessionId) {
        crate::voice::control(id, "cancelVoice");
    }
    fn request(&self, command: Command) -> UiFuture<Response> {
        let worker = self.worker.clone();
        Box::pin(async move { worker.request(command).await })
    }

    fn save_csv(&self, csv: CsvExport) -> UiFuture<Option<String>> {
        Box::pin(async move {
            let started = on_activity(move |env, activity| {
                let name = env.new_string(csv.filename)?;
                let data = env.byte_array_from_slice(csv.contents.as_bytes())?;
                env.call_method(
                    activity,
                    "beginCsvExport",
                    "(Ljava/lang/String;[B)Z",
                    &[(&name).into(), (&data).into()],
                )?
                .z()
            })
            .await?;
            if !started {
                return Err(AppError::Input("เปิดหน้าต่างบันทึกไฟล์ไม่ได้ กรุณาลองใหม่".into()));
            }
            loop {
                let state = on_activity(|env, activity| {
                    env.call_method(activity, "csvExportState", "()I", &[])?.i()
                })
                .await?;
                match state {
                    1 => tokio::time::sleep(Duration::from_millis(250)).await,
                    2 => {
                        let saved_name = on_activity(|env, activity| {
                            let name = env
                                .call_method(
                                    activity,
                                    "csvExportName",
                                    "()Ljava/lang/String;",
                                    &[],
                                )?
                                .l()?;
                            Ok(String::from(env.get_string(&name.into())?))
                        })
                        .await?;
                        return Ok(Some(saved_name));
                    }
                    3 => return Ok(None),
                    _ => return Err(AppError::Input("บันทึกไฟล์ไม่สำเร็จ กรุณาเลือกที่บันทึกอีกครั้ง".into())),
                }
            }
        })
    }

    fn scan_receipt(
        &self,
        _file: dioxus::html::FileData,
        _cancel: Arc<AtomicBool>,
    ) -> UiFuture<(ReceiptImage, String)> {
        Box::pin(async {
            Err(AppError::Input("กรุณาใช้ปุ่มเลือกรูปใบเสร็จของ Android".into()))
        })
    }
    fn uses_native_receipt_picker(&self) -> bool {
        true
    }
    fn pick_receipts(&self, cancel: Arc<AtomicBool>) -> UiFuture<Vec<ledger_ui::ReceiptScan>> {
        crate::receipt::pick(cancel)
    }
}
