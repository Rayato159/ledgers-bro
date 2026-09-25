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
    directory: PathBuf,
    updater: ledger_infrastructure::AppUpdater,
    alerts: Option<ledger_infrastructure::AlertStore>,
    worker: Option<LedgerWorker>,
    profiles: ledger_infrastructure::ProfileWorker,
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
    let (cache, version): (String, String) = on_activity(|env, activity| {
        let cache = env
            .call_method(activity, "getCacheDir", "()Ljava/io/File;", &[])?
            .l()?;
        let path = env
            .call_method(cache, "getAbsolutePath", "()Ljava/lang/String;", &[])?
            .l()?;
        let path: String = env.get_string(&path.into())?.into();
        let version = env
            .call_method(activity, "appVersion", "()Ljava/lang/String;", &[])?
            .l()?;
        Ok((path, env.get_string(&version.into())?.into()))
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
                    directory: directory.clone(),
                    updater: ledger_infrastructure::AppUpdater::new(
                        PathBuf::from(cache).join("updates"),
                        version,
                        if cfg!(target_arch = "aarch64") {
                            ledger_application::UpdatePlatform::AndroidArm64
                        } else {
                            ledger_application::UpdatePlatform::AndroidX64
                        },
                    ),
                    alerts: None,
                    worker: None,
                    profiles: ledger_infrastructure::ProfileWorker::start(&directory)?,
                    model: ModelWorker::start(directory.join("models"))?,
                })))
            })();
            let _ = send.send(result);
        })
        .map_err(|_| AppError::WorkerStopped)?;
    receive.await.map_err(|_| AppError::WorkerStopped)?
}

impl UiGateway for AndroidGateway {
    fn supports_updates(&self) -> bool {
        true
    }
    fn check_update(&self) -> UiFuture<ledger_application::UpdateCheck> {
        let updater = self.updater.clone();
        Box::pin(async move { updater.check().await })
    }
    fn download_update(
        &self,
        release: ledger_application::AppRelease,
        operation: ledger_application::UpdateOperation,
    ) -> UiFuture<()> {
        let updater = self.updater.clone();
        Box::pin(async move { updater.download(release, operation).await })
    }
    fn install_update(&self) -> UiFuture<ledger_application::UpdateInstall> {
        let updater = self.updater.clone();
        Box::pin(async move {
            let (path, _) = updater.installer().await?;
            let status = on_activity(move |env, activity| {
                let path = env.new_string(path.to_string_lossy())?;
                env.call_method(
                    activity,
                    "installUpdate",
                    "(Ljava/lang/String;)I",
                    &[(&path).into()],
                )?
                .i()
            })
            .await?;
            match status {
                1 => Ok(ledger_application::UpdateInstall::InstallerOpened),
                2 => Ok(ledger_application::UpdateInstall::PermissionRequired),
                _ => Err(AppError::Input(
                    "APK ไม่ผ่านการตรวจแพ็กเกจ รุ่น หรือลายเซ็น หรือเปิดตัวติดตั้งไม่ได้".into(),
                )),
            }
        })
    }
    fn alert_preferences(&self) -> UiFuture<ledger_application::AlertPreferences> {
        let alerts = self.alerts.clone();
        Box::pin(async move {
            alerts
                .ok_or_else(ledger_application::login_required)?
                .load()
                .await
        })
    }
    fn save_alert_preferences(&self, prefs: ledger_application::AlertPreferences) -> UiFuture<()> {
        let alerts = self.alerts.clone();
        Box::pin(async move {
            alerts
                .ok_or_else(ledger_application::login_required)?
                .save(prefs)
                .await
        })
    }
    fn enable_system_notifications(&self) -> UiFuture<bool> {
        Box::pin(on_activity(|env, activity| {
            env.call_method(activity, "enableNotifications", "()Z", &[])?
                .z()
        }))
    }
    fn system_notification(&self, title: String, body: String) -> UiFuture<()> {
        Box::pin(async move {
            let shown = on_activity(move |env, activity| {
                let title = env.new_string(title)?;
                let body = env.new_string(body)?;
                env.call_method(
                    activity,
                    "showNotification",
                    "(Ljava/lang/String;Ljava/lang/String;)Z",
                    &[(&title).into(), (&body).into()],
                )?
                .z()
            })
            .await?;
            if shown {
                Ok(())
            } else {
                Err(AppError::Input(
                    "แสดงการแจ้งเตือนระบบไม่ได้ กรุณาตรวจสิทธิ์การแจ้งเตือน".into(),
                ))
            }
        })
    }
    fn profiles(
        &self,
        command: ledger_application::ProfileCommand,
    ) -> UiFuture<ledger_application::ProfileResponse> {
        let worker = self.profiles.clone();
        Box::pin(async move { worker.request(command).await })
    }
    fn authenticated(&self, token: &str) -> Result<Gateway, AppError> {
        Ok(Gateway(Arc::new(Self {
            directory: self.directory.clone(),
            updater: self.updater.clone(),
            alerts: Some(ledger_infrastructure::AlertStore::new(
                &self.directory,
                &self.profiles.profile_key(token)?,
            )?),
            worker: Some(self.profiles.ledger(token)?),
            profiles: self.profiles.clone(),
            model: self.model.clone(),
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
    fn delete_model(&self, id: ledger_application::LocalModelId) -> UiFuture<()> {
        let model = self.model.clone();
        Box::pin(async move { model.delete(id).await })
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
            let started = on_activity(move |env, activity| {
                let name = env.new_string(document.filename)?;
                let mime = env.new_string(document.mime)?;
                let data = env.byte_array_from_slice(&document.bytes)?;
                env.call_method(
                    activity,
                    "beginDocumentExport",
                    "(Ljava/lang/String;Ljava/lang/String;[B)Z",
                    &[(&name).into(), (&mime).into(), (&data).into()],
                )?
                .z()
            })
            .await?;
            if !started {
                return Err(AppError::Input("เปิดหน้าต่างบันทึกไฟล์ไม่ได้ กรุณาลองใหม่".into()));
            }
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                let (state, saved_name) = on_activity(|env, activity| {
                    let state = env
                        .call_method(activity, "csvExportState", "()I", &[])?
                        .i()?;
                    let name = if state == 2 {
                        let name = env
                            .call_method(activity, "csvExportName", "()Ljava/lang/String;", &[])?
                            .l()?;
                        String::from(env.get_string(&name.into())?)
                    } else {
                        String::new()
                    };
                    Ok((state, name))
                })
                .await?;
                match state {
                    1 => {}
                    2 => return Ok(Some(saved_name)),
                    3 => return Ok(None),
                    _ => return Err(AppError::Input("บันทึกไฟล์ไม่สำเร็จ กรุณาเลือกที่บันทึกอีกครั้ง".into())),
                }
            }
        })
    }

    fn pick_document(
        &self,
        kind: ledger_application::ImportFileKind,
    ) -> UiFuture<Option<Arc<[u8]>>> {
        Box::pin(async move {
            let started = on_activity(move |env, activity| {
                env.call_method(
                    activity,
                    "beginDocumentImport",
                    "(I)Z",
                    &[(kind.max_bytes() as i32).into()],
                )?
                .z()
            })
            .await?;
            if !started {
                return Err(AppError::Input("เปิดหน้าต่างเลือกไฟล์ไม่ได้ กรุณาลองอีกครั้ง".into()));
            }
            loop {
                tokio::time::sleep(Duration::from_millis(250)).await;
                // Read completion and consume its payload in one UI-thread dispatch.
                // Splitting these into consecutive Wry dispatches can strand the
                // second callback while the activity resumes from the picker.
                let (state, bytes) = on_activity(|env, activity| {
                    let state = env
                        .call_method(activity, "documentImportState", "()I", &[])?
                        .i()?;
                    let bytes = if state == 2 {
                        let array = env
                            .call_method(activity, "takeImportedDocument", "()[B", &[])?
                            .l()?;
                        env.convert_byte_array(jni::objects::JByteArray::from(array))?
                    } else {
                        Vec::new()
                    };
                    Ok((state, bytes))
                })
                .await?;
                match state {
                    1 => {}
                    2 => {
                        if bytes.len() > kind.max_bytes() {
                            return Err(AppError::Input("ไฟล์ใหญ่เกินขนาดที่รองรับ".into()));
                        }
                        return Ok(Some(bytes.into()));
                    }
                    3 => return Ok(None),
                    _ => return Err(AppError::Input("อ่านไฟล์ไม่ได้ หรือไฟล์ใหญ่เกินขนาดที่รองรับ".into())),
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
