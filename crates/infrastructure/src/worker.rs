use crate::SqliteLedger;
use futures_channel::oneshot;
use ledger_application::*;
use ledger_domain::*;
use std::{
    path::PathBuf,
    sync::mpsc::{self, SyncSender, TrySendError},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
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
    fn receivable_id(&self) -> Result<ReceivableId, AppError> {
        Ok(ReceivableId::from_uuid(Uuid::new_v4())?)
    }
    fn recurring_id(&self) -> Result<RecurringId, AppError> {
        Ok(RecurringId::from_uuid(Uuid::new_v4())?)
    }
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
    active: Option<Arc<AtomicBool>>,
    reply: oneshot::Sender<Result<Response, AppError>>,
}

#[derive(Clone)]
pub struct LedgerWorker {
    sender: SyncSender<Envelope>,
    active: Option<Arc<AtomicBool>>,
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
                    let response = if envelope
                        .active
                        .as_ref()
                        .is_some_and(|a| !a.load(Ordering::Acquire))
                    {
                        Err(login_required())
                    } else {
                        match &mut application {
                            Ok(app) => app.execute(envelope.command),
                            Err(error) => Err(error.clone().into()),
                        }
                    };
                    // A closed receiver means the view was disposed; the transaction still
                    // has exactly-once retry semantics through its submission ID.
                    let _ = envelope.reply.send(response);
                }
            })
            .map_err(|_| AppError::WorkerStopped)?;
        Ok(Self {
            sender,
            active: None,
        })
    }
    pub(crate) fn guarded(mut self, active: Arc<AtomicBool>) -> Self {
        self.active = Some(active);
        self
    }
    pub async fn request(&self, command: Command) -> Result<Response, AppError> {
        if self
            .active
            .as_ref()
            .is_some_and(|a| !a.load(Ordering::Acquire))
        {
            return Err(login_required());
        }
        let (reply, receiver) = oneshot::channel();
        self.sender
            .try_send(Envelope {
                command,
                reply,
                active: self.active.clone(),
            })
            .map_err(|error| match error {
                TrySendError::Full(_) => AppError::Busy,
                TrySendError::Disconnected(_) => AppError::WorkerStopped,
            })?;
        let result = receiver.await.map_err(|_| AppError::WorkerStopped)?;
        if self
            .active
            .as_ref()
            .is_some_and(|a| !a.load(Ordering::Acquire))
        {
            return Err(login_required());
        }
        result
    }
}
