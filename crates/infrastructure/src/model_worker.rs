use crate::{LedgerWorker, local_model::LocalModel};
use futures_channel::oneshot;
use ledger_application::{
    AppError, Command, LocalModelId, LocalModelInfo, ModelAvailability, ModelDevice, ModelFit,
    ModelOperation, ModelPhase, ModelSettingsSnapshot, QuickEntryModel, Response, model_fit,
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::Ordering,
        mpsc::{self, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};

pub const LOCAL_MODEL_FILENAME: &str = "Qwen3-0.6B-Q4_K_M.gguf";
pub const LOCAL_MODEL_BYTES: u64 = 396_705_472;

enum Work {
    Status,
    Install(LocalModelId),
    Delete(LocalModelId),
    Settings,
    Propose(String),
}
enum Answer {
    Status(ModelAvailability),
    Installed,
    Deleted,
    Settings(ModelSettingsSnapshot),
    Proposal(String),
}
struct Envelope {
    work: Work,
    operation: ModelOperation,
    reply: oneshot::Sender<Result<Answer, AppError>>,
}
#[derive(Clone)]
pub struct ModelWorker {
    sender: SyncSender<Envelope>,
}

fn model_error(message: &str) -> AppError {
    AppError::Input(message.into())
}
fn check_cancel(operation: &ModelOperation) -> Result<(), AppError> {
    if operation.cancelled.load(Ordering::Relaxed) {
        Err(model_error("ยกเลิก AI ในเครื่องแล้ว"))
    } else {
        Ok(())
    }
}

impl ModelWorker {
    pub fn start(directory: PathBuf) -> Result<Self, AppError> {
        let (sender, receiver) = mpsc::sync_channel::<Envelope>(1);
        std::thread::Builder::new()
            .name("local-interpretation".into())
            .spawn(move || {
                let mut model: Option<LocalModel> = None;
                let mut selected = read_selection(&directory);
                loop {
                    let envelope = match receiver.recv_timeout(Duration::from_secs(60)) {
                        Ok(envelope) => envelope,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            model = None;
                            continue;
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    let result = (|| {
                        check_cancel(&envelope.operation)?;
                        let spec = selected.info();
                        let path = directory.join(spec.filename);
                        match envelope.work {
                            Work::Delete(id) => {
                                if id == selected {
                                    model = None;
                                }
                                remove_model(&directory, id)?;
                                Ok(Answer::Deleted)
                            }
                            Work::Status => Ok(Answer::Status(
                                if verified(&path, &spec, &envelope.operation).is_ok() {
                                    ModelAvailability::Installed
                                } else {
                                    ModelAvailability::Missing
                                },
                            )),
                            Work::Settings => {
                                Ok(Answer::Settings(settings_snapshot(&directory, selected)))
                            }
                            Work::Install(id) => {
                                // Release old weights before estimating or loading another model.
                                model = None;
                                let spec = id.info();
                                let snapshot = settings_snapshot(&directory, selected);
                                match model_fit(
                                    &spec,
                                    &snapshot.device,
                                    !snapshot.downloaded.contains(&id),
                                ) {
                                    ModelFit::InsufficientDisk => {
                                        return Err(model_error("พื้นที่ว่างไม่พอสำหรับโมเดลนี้"));
                                    }
                                    ModelFit::InsufficientMemory => {
                                        return Err(model_error(
                                            "หน่วยความจำเครื่องไม่พอ เลือกโมเดลที่เล็กลง",
                                        ));
                                    }
                                    _ => {}
                                }
                                install(&directory, &spec, &envelope.operation)?;
                                check_cancel(&envelope.operation)?;
                                write_selection(&directory, id)?;
                                selected = id;
                                Ok(Answer::Installed)
                            }
                            Work::Propose(source) => {
                                if model.is_none() {
                                    envelope.operation.set_phase(ModelPhase::Verifying);
                                    verified(&path, &spec, &envelope.operation)?;
                                    if device_snapshot(&directory)
                                        .available_memory
                                        .is_some_and(|free| free < spec.working_memory_bytes)
                                    {
                                        return Err(model_error(
                                            "RAM ว่างไม่พอเปิดโมเดลนี้ ปิดแอปอื่นหรือเลือกโมเดลที่เล็กลง",
                                        ));
                                    }
                                    check_cancel(&envelope.operation)?;
                                    envelope.operation.set_phase(ModelPhase::Loading);
                                    model = Some(LocalModel::load(&path)?);
                                }
                                let engine = model
                                    .as_mut()
                                    .ok_or_else(|| model_error("เปิด AI ในเครื่องไม่ได้"))?;
                                engine
                                    .propose(&source, &envelope.operation)
                                    .map(Answer::Proposal)
                            }
                        }
                    })();
                    let _ = envelope.reply.send(result);
                }
            })
            .map_err(|_| model_error("เริ่ม AI ในเครื่องไม่ได้"))?;
        Ok(Self { sender })
    }
    async fn request(&self, work: Work, operation: ModelOperation) -> Result<Answer, AppError> {
        let (reply, receive) = oneshot::channel();
        self.sender
            .try_send(Envelope {
                work,
                operation,
                reply,
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => AppError::Busy,
                TrySendError::Disconnected(_) => model_error("AI หยุดทำงาน กรุณาเปิดแอปใหม่"),
            })?;
        receive
            .await
            .map_err(|_| model_error("AI หยุดทำงาน กรุณาเปิดแอปใหม่"))?
    }
    pub async fn availability(&self) -> Result<ModelAvailability, AppError> {
        match self
            .request(Work::Status, ModelOperation::default())
            .await?
        {
            Answer::Status(status) => Ok(status),
            _ => Err(model_error("อ่านสถานะ AI ไม่สำเร็จ")),
        }
    }
    pub async fn delete(&self, id: LocalModelId) -> Result<(), AppError> {
        match self
            .request(Work::Delete(id), ModelOperation::default())
            .await?
        {
            Answer::Deleted => Ok(()),
            _ => Err(model_error("ลบโมเดลไม่สำเร็จ")),
        }
    }
    pub async fn install(&self, operation: ModelOperation) -> Result<(), AppError> {
        self.activate(LocalModelId::Small, operation).await
    }
    pub async fn settings(&self) -> Result<ModelSettingsSnapshot, AppError> {
        match self
            .request(Work::Settings, ModelOperation::default())
            .await?
        {
            Answer::Settings(snapshot) => Ok(snapshot),
            _ => Err(model_error("อ่านการตั้งค่า AI ไม่สำเร็จ")),
        }
    }
    pub async fn activate(
        &self,
        id: LocalModelId,
        operation: ModelOperation,
    ) -> Result<(), AppError> {
        match self.request(Work::Install(id), operation).await? {
            Answer::Installed => Ok(()),
            _ => Err(model_error("ติดตั้ง AI ไม่สำเร็จ")),
        }
    }
    pub async fn propose(
        &self,
        source: String,
        operation: ModelOperation,
    ) -> Result<String, AppError> {
        match self.request(Work::Propose(source), operation).await? {
            Answer::Proposal(output) => Ok(output),
            _ => Err(model_error("อ่านรายการไม่สำเร็จ")),
        }
    }
    /// Exact commands remain instant. Free prose uses local inference, then the
    /// application validates against current accounts. Neither path commits.
    pub async fn resolve(
        &self,
        ledger: &LedgerWorker,
        source: String,
        operation: ModelOperation,
    ) -> Result<Response, AppError> {
        check_cancel(&operation)?;
        if ledger_application::is_receipt_prompt(&source) {
            return ledger.request(Command::Resolve(source)).await;
        }
        if source.trim().is_empty() || source.chars().count() > 1000 {
            return Err(model_error("พิมพ์รายการไม่เกิน 1,000 ตัวอักษร"));
        }
        match ledger.request(Command::Resolve(source.clone())).await {
            Ok(response) => Ok(response),
            Err(error @ (AppError::Input(_) | AppError::Rule(_))) => {
                // The model contract describes alternatives for ONE transaction.
                // Never let failed multi-entry parsing fall back to saving a prefix.
                if ledger_application::is_prompt_action(&source)
                    || source.contains(';')
                    || source.contains("และ")
                    || source.contains("แล้ว")
                    || source.contains('\n')
                    || source.contains("ตามลำดับ")
                {
                    return Err(error);
                }
                let Response::Dashboard(view) = ledger.request(Command::Load).await? else {
                    return Err(AppError::WorkerStopped);
                };
                let source = ledger_application::normalize_prompt_currency(&source, view.currency)?;
                ledger_application::check_model_source(&source)?;
                let output = self.propose(source.clone(), operation.clone()).await?;
                check_cancel(&operation)?;
                ledger
                    .request(Command::ResolveModel { source, output })
                    .await
            }
            Err(error) => Err(error),
        }
    }
}

fn verified(
    path: &Path,
    spec: &LocalModelInfo,
    operation: &ModelOperation,
) -> Result<(), AppError> {
    let invalid =
        || model_error("ยังไม่มีโมเดล AI ที่พร้อมใช้ กดดาวน์โหลด AI ในเครื่องก่อน หรือใช้ตัวอย่างคำสั่งได้ทันที");
    let mut file = File::open(path).map_err(|_| invalid())?;
    if file.metadata().map_err(|_| invalid())?.len() != spec.bytes {
        return Err(invalid());
    }
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancel(operation)?;
        let count = file.read(&mut buffer).map_err(|_| invalid())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if format!("{:x}", digest.finalize()) != spec.sha256 {
        return Err(invalid());
    }
    Ok(())
}

fn install(
    directory: &Path,
    spec: &LocalModelInfo,
    operation: &ModelOperation,
) -> Result<(), AppError> {
    let fail = || model_error("ดาวน์โหลด AI ไม่สำเร็จ ตรวจอินเทอร์เน็ตและพื้นที่ว่างแล้วลองใหม่");
    std::fs::create_dir_all(directory).map_err(|_| fail())?;
    let path = directory.join(spec.filename);
    if verified(&path, spec, operation).is_ok() {
        return Ok(());
    }
    check_cancel(operation)?;
    // A corrupt old file is kept until the replacement is fully validated.
    if device_snapshot(directory)
        .free_disk
        .is_some_and(|free| free < spec.bytes + 256 * 1024 * 1024)
    {
        return Err(model_error("พื้นที่ว่างไม่พอสำหรับโมเดลนี้"));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(|_| fail())?;
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| fail())?;
    let mut total = 0_u64;
    let started = Instant::now();
    let mut buffer = [0_u8; 64 * 1024];
    // Bounded range requests avoid a short whole-file timeout on multi-GB weights.
    // Cancellation and stalls are bounded by one 30-second request.
    while total < spec.bytes {
        check_cancel(operation)?;
        if started.elapsed() > Duration::from_secs(7200) {
            return Err(fail());
        }
        let end = (total + 8 * 1024 * 1024).min(spec.bytes) - 1;
        let mut response = client
            .get(spec.url)
            .header(reqwest::header::RANGE, format!("bytes={total}-{end}"))
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|_| fail())?;
        if response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
            return Err(fail());
        }
        loop {
            check_cancel(operation)?;
            let count = response.read(&mut buffer).map_err(|_| fail())?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > end + 1 {
                return Err(fail());
            }
            temporary.write_all(&buffer[..count]).map_err(|_| fail())?;
            operation.downloaded_bytes.store(total, Ordering::Relaxed);
        }
        if total != end + 1 {
            return Err(fail());
        }
    }
    temporary.as_file().sync_all().map_err(|_| fail())?;
    operation.set_phase(ModelPhase::Verifying);
    verified(temporary.path(), spec, operation)?;
    check_cancel(operation)?;
    temporary.persist(&path).map_err(|_| fail())?;
    Ok(())
}

fn read_selection(directory: &Path) -> LocalModelId {
    std::fs::read_to_string(directory.join("selected-model.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(LocalModelId::Small)
}
fn write_selection(directory: &Path, id: LocalModelId) -> Result<(), AppError> {
    let fail = || model_error("บันทึกโมเดลที่เลือกไม่สำเร็จ โมเดลเดิมยังเป็นตัวหลัก");
    let mut temp = tempfile::NamedTempFile::new_in(directory).map_err(|_| fail())?;
    serde_json::to_writer(temp.as_file_mut(), &id).map_err(|_| fail())?;
    temp.as_file().sync_all().map_err(|_| fail())?;
    temp.persist(directory.join("selected-model.json"))
        .map_err(|_| fail())?;
    Ok(())
}
fn device_snapshot(directory: &Path) -> ModelDevice {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let nonzero = |value| if value == 0 { None } else { Some(value) };
    let existing = directory.ancestors().find(|p| p.exists());
    ModelDevice {
        total_memory: nonzero(system.total_memory()),
        available_memory: nonzero(system.available_memory()),
        free_disk: existing.and_then(|p| fs2::available_space(p).ok()),
        cpu_threads: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
        mobile: cfg!(any(target_os = "android", target_os = "ios")),
    }
}
fn settings_snapshot(directory: &Path, selected: LocalModelId) -> ModelSettingsSnapshot {
    ModelSettingsSnapshot {
        selected,
        downloaded: LocalModelId::ALL
            .into_iter()
            .filter(|id| {
                let spec = id.info();
                std::fs::metadata(directory.join(spec.filename))
                    .is_ok_and(|m| m.is_file() && m.len() == spec.bytes)
            })
            .collect(),
        device: device_snapshot(directory),
    }
}

fn remove_model(directory: &Path, id: LocalModelId) -> Result<(), AppError> {
    let path = directory.join(id.info().filename);
    if !path.exists() {
        return Ok(());
    }
    let root = directory
        .canonicalize()
        .map_err(|_| model_error("เปิดโฟลเดอร์โมเดลไม่ได้"))?;
    let resolved = path
        .canonicalize()
        .map_err(|_| model_error("ตรวจไฟล์โมเดลไม่ได้"))?;
    if resolved.parent() != Some(root.as_path()) || !resolved.is_file() {
        return Err(model_error("ไฟล์โมเดลอยู่นอกโฟลเดอร์ที่อนุญาต"));
    }
    std::fs::remove_file(&path)
        .map_err(|_| model_error("ลบโมเดลไม่สำเร็จ กรุณาปิดการใช้งานโมเดลแล้วลองใหม่"))
}
