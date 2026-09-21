use crate::SqliteLedger;
use futures_channel::oneshot;
use ledger_application::*;
use ledger_domain::*;
use std::{
    path::PathBuf,
    sync::mpsc::{self, SyncSender, TrySendError},
};
use uuid::Uuid;

pub struct SystemClock;
impl Clock for SystemClock {
    fn today(&self) -> Result<EntryDate, AppError> {
        Ok(EntryDate::new(chrono::Local::now().date_naive())?)
    }
}
pub struct RandomIds;
impl IdSource for RandomIds {
    fn account_id(&self) -> Result<AccountId, AppError> {
        Ok(AccountId::from_uuid(Uuid::new_v4())?)
    }
    fn entry_id(&self) -> Result<EntryId, AppError> {
        Ok(EntryId::from_uuid(Uuid::new_v4())?)
    }
    fn submission_id(&self) -> Result<SubmissionId, AppError> {
        Ok(SubmissionId::from_uuid(Uuid::new_v4())?)
    }
}

struct Envelope {
    command: Command,
    reply: oneshot::Sender<Result<Response, AppError>>,
}

#[derive(Clone)]
pub struct LedgerWorker {
    sender: SyncSender<Envelope>,
}
impl LedgerWorker {
    /// The connection is opened and used only on this dedicated thread.
    /// No mutex guarding a connection on the UI thread, and a bounded queue.
    pub fn start(path: PathBuf) -> Result<Self, AppError> {
        let (sender, receiver) = mpsc::sync_channel::<Envelope>(32);
        std::thread::Builder::new()
            .name("ledger-storage".into())
            .spawn(move || {
                let storage = SqliteLedger::open(&path);
                let mut application =
                    storage.map(|repo| LedgerApplication::new(repo, SystemClock, RandomIds));
                while let Ok(envelope) = receiver.recv() {
                    let response = match &mut application {
                        Ok(app) => app.execute(envelope.command),
                        Err(error) => Err(error.clone().into()),
                    };
                    // A closed receiver means the view was disposed; the transaction still
                    // has exactly-once retry semantics through its submission ID.
                    let _ = envelope.reply.send(response);
                }
            })
            .map_err(|_| AppError::WorkerStopped)?;
        Ok(Self { sender })
    }
    pub async fn request(&self, command: Command) -> Result<Response, AppError> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .try_send(Envelope { command, reply })
            .map_err(|error| match error {
                TrySendError::Full(_) => AppError::Busy,
                TrySendError::Disconnected(_) => AppError::WorkerStopped,
            })?;
        receiver.await.map_err(|_| AppError::WorkerStopped)?
    }
}
