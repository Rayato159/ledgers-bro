use crate::{LedgerWorker, local_model::LocalModel};
use futures_channel::oneshot;
use ledger_application::{
    AppError, Command, ModelAvailability, ModelOperation, ModelPhase, QuickEntryModel, Response,
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
const HASH: &str = "ac2d97712095a558e31573f62f466a3f9d93990898b0ec79d7c974c1780d524a";
const URL: &str = "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/resolve/50968a4468ef4233ed78cd7c3de230dd1d61a56b/Qwen3-0.6B-Q4_K_M.gguf";

enum Work {
    Status,
    Install,
    Propose(String),
}
enum Answer {
    Status(ModelAvailability),
    Installed,
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
                        let path = directory.join(LOCAL_MODEL_FILENAME);
                        match envelope.work {
                            Work::Status => Ok(Answer::Status(
                                if verified(&path, &envelope.operation).is_ok() {
                                    ModelAvailability::Installed
                                } else {
                                    ModelAvailability::Missing
                                },
                            )),
                            Work::Install => {
                                model = None;
                                install(&directory, &envelope.operation)?;
                                Ok(Answer::Installed)
                            }
                            Work::Propose(source) => {
                                if model.is_none() {
                                    envelope.operation.set_phase(ModelPhase::Verifying);
                                    verified(&path, &envelope.operation)?;
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
    pub async fn install(&self, operation: ModelOperation) -> Result<(), AppError> {
        match self.request(Work::Install, operation).await? {
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
        if source.trim().is_empty() || source.chars().count() > 1000 {
            return Err(model_error("พิมพ์รายการไม่เกิน 1,000 ตัวอักษร"));
        }
        match ledger.request(Command::Resolve(source.clone())).await {
            Ok(response) => Ok(response),
            Err(error @ (AppError::Input(_) | AppError::Rule(_))) => {
                // The model contract describes alternatives for ONE transaction.
                // Never let failed multi-entry parsing fall back to saving a prefix.
                if source.contains("และ") || source.contains('\n') || source.contains("ตามลำดับ")
                {
                    return Err(error);
                }
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

fn verified(path: &Path, operation: &ModelOperation) -> Result<(), AppError> {
    let invalid =
        || model_error("ยังไม่มีโมเดล AI ที่พร้อมใช้ กดดาวน์โหลด AI ในเครื่องก่อน หรือใช้ตัวอย่างคำสั่งได้ทันที");
    let mut file = File::open(path).map_err(|_| invalid())?;
    if file.metadata().map_err(|_| invalid())?.len() != LOCAL_MODEL_BYTES {
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
    if format!("{:x}", digest.finalize()) != HASH {
        return Err(invalid());
    }
    Ok(())
}

fn install(directory: &Path, operation: &ModelOperation) -> Result<(), AppError> {
    let fail = || model_error("ดาวน์โหลด AI ไม่สำเร็จ ตรวจอินเทอร์เน็ตและพื้นที่ว่างแล้วลองใหม่");
    std::fs::create_dir_all(directory).map_err(|_| fail())?;
    let path = directory.join(LOCAL_MODEL_FILENAME);
    if verified(&path, operation).is_ok() {
        return Ok(());
    }
    check_cancel(operation)?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory).map_err(|_| fail())?;
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|_| fail())?;
    // Only this pinned model URL is sent. No financial text, identity or auth token.
    let mut response = client
        .get(URL)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|_| fail())?;
    let mut total = 0_u64;
    let started = Instant::now();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        check_cancel(operation)?;
        if started.elapsed() > Duration::from_secs(900) {
            return Err(fail());
        }
        let count = response.read(&mut buffer).map_err(|_| fail())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > LOCAL_MODEL_BYTES {
            return Err(fail());
        }
        temporary.write_all(&buffer[..count]).map_err(|_| fail())?;
        operation.downloaded_bytes.store(total, Ordering::Relaxed);
    }
    temporary.as_file().sync_all().map_err(|_| fail())?;
    verified(temporary.path(), operation)?;
    check_cancel(operation)?;
    temporary.persist(&path).map_err(|_| fail())?;
    Ok(())
}
