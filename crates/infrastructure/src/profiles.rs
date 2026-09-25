use crate::remembered_login::RememberedLoginStore;
use crate::{LedgerWorker, SqliteLedger};
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier,
    password_hash::{PasswordHash, SaltString},
};
use futures_channel::oneshot;
use ledger_application::*;
use rand_core::OsRng;
use rusqlite::{Connection, OptionalExtension, params};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zeroize::Zeroizing;

fn profile_error() -> AppError {
    AppError::Input("เปิดข้อมูลผู้ใช้ไม่สำเร็จ กรุณาลองอีกครั้ง".into())
}
fn invalid_login() -> AppError {
    AppError::Input("ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง".into())
}
fn now() -> Result<i64, AppError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| profile_error())?
            .as_secs(),
    )
    .map_err(|_| profile_error())
}
fn hash_password(password: &str) -> Result<String, AppError> {
    validate_new_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|v| v.to_string())
        .map_err(|_| profile_error())
}

struct ActiveSession {
    session: ProfileSession,
    worker: LedgerWorker,
    active: Arc<AtomicBool>,
}
type SharedSession = Arc<Mutex<Option<ActiveSession>>>;
struct Registry {
    connection: Connection,
    directory: PathBuf,
    active: SharedSession,
    remembered: RememberedLoginStore,
}
impl Registry {
    fn open(directory: PathBuf, active: SharedSession) -> Result<Self, AppError> {
        std::fs::create_dir_all(&directory).map_err(|_| profile_error())?;
        let mut connection = Connection::open(directory.join("user-profiles.sqlite3"))
            .map_err(|_| profile_error())?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|_| profile_error())?;
        connection
            .execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON;")
            .map_err(|_| profile_error())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| profile_error())?;
        let version: i64 = tx
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|_| profile_error())?;
        if version > 2 {
            return Err(AppError::Input(
                "ข้อมูลผู้ใช้มาจากแอปรุ่นใหม่กว่า กรุณาอัปเดตแอป".into(),
            ));
        }
        if version == 0 {
            let tables: i64 = tx.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |r| r.get(0)).map_err(|_| profile_error())?;
            if tables != 0 {
                return Err(profile_error());
            }
            tx.execute_batch("CREATE TABLE profiles(id TEXT PRIMARY KEY, username TEXT NOT NULL, name_key TEXT NOT NULL UNIQUE, password_hash TEXT, legacy INTEGER NOT NULL CHECK(legacy IN (0,1)), failures INTEGER NOT NULL DEFAULT 0, locked_until INTEGER NOT NULL DEFAULT 0); CREATE UNIQUE INDEX one_legacy_profile ON profiles(legacy) WHERE legacy=1; PRAGMA user_version=1;").map_err(|_| profile_error())?;
            // Adopt the existing database by reference. Do not move/copy a live WAL
            // database or overwrite it when the app is upgraded.
            if directory.join("ledger.sqlite3").is_file() {
                tx.execute("INSERT INTO profiles(id,username,name_key,legacy) VALUES (?1,'Lookhin','lookhin',1)", [Uuid::new_v4().to_string()]).map_err(|_| profile_error())?;
            }
        }
        if version < 2 {
            tx.execute_batch("CREATE TABLE remembered_login(singleton INTEGER PRIMARY KEY CHECK(singleton=1), profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE, token_hash BLOB NOT NULL CHECK(length(token_hash)=32), issued_at INTEGER NOT NULL, expires_at INTEGER NOT NULL, last_used_at INTEGER NOT NULL); PRAGMA user_version=2;").map_err(|_| profile_error())?;
        }
        tx.commit().map_err(|_| profile_error())?;
        Ok(Self {
            connection,
            remembered: RememberedLoginStore::new(&directory),
            directory,
            active,
        })
    }
    fn list(&self) -> Result<Vec<UserProfile>, AppError> {
        let mut statement = self
            .connection
            .prepare("SELECT id,username,password_hash IS NULL FROM profiles ORDER BY name_key,id")
            .map_err(|_| profile_error())?;
        statement
            .query_map([], |r| {
                Ok(UserProfile {
                    id: r.get(0)?,
                    username: r.get(1)?,
                    needs_password: r.get(2)?,
                })
            })
            .map_err(|_| profile_error())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| profile_error())
    }
    fn profile(&self, id: &str) -> Result<UserProfile, AppError> {
        self.list()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(invalid_login)
    }
    fn unique(&self, name: &Username, except: Option<&str>) -> Result<(), AppError> {
        let existing: Option<String> = self
            .connection
            .query_row(
                "SELECT id FROM profiles WHERE name_key=?1",
                [name.key()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| profile_error())?;
        if existing.as_deref().is_some_and(|id| Some(id) != except) {
            return Err(AppError::Input("ชื่อผู้ใช้นี้มีอยู่แล้ว".into()));
        }
        Ok(())
    }
    fn verify(&self, id: &str, password: &str) -> Result<(), AppError> {
        if password.len() > 256 {
            return Err(invalid_login());
        }
        let (hash, failures, locked): (Option<String>, u32, i64) = self
            .connection
            .query_row(
                "SELECT password_hash,failures,locked_until FROM profiles WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|_| profile_error())?
            .ok_or_else(invalid_login)?;
        let time = now()?;
        if locked > time {
            return Err(AppError::Input(
                "ลองรหัสผ่านหลายครั้งเกินไป กรุณารอ 30 วินาที".into(),
            ));
        }
        let verified = hash
            .as_deref()
            .and_then(|h| PasswordHash::new(h).ok())
            .is_some_and(|hash| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &hash)
                    .is_ok()
            });
        if !verified {
            let next = failures.saturating_add(1).min(5);
            self.connection
                .execute(
                    "UPDATE profiles SET failures=?1,locked_until=?2 WHERE id=?3",
                    params![next, if next >= 5 { time + 30 } else { 0 }, id],
                )
                .map_err(|_| profile_error())?;
            return Err(invalid_login());
        }
        self.connection
            .execute(
                "UPDATE profiles SET failures=0,locked_until=0 WHERE id=?1",
                [id],
            )
            .map_err(|_| profile_error())?;
        Ok(())
    }
    fn ledger_path(&self, id: &str) -> Result<PathBuf, AppError> {
        let parsed = Uuid::parse_str(id).map_err(|_| profile_error())?;
        let legacy: bool = self
            .connection
            .query_row("SELECT legacy FROM profiles WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| profile_error())?;
        Ok(if legacy {
            self.directory.join("ledger.sqlite3")
        } else {
            self.directory.join(format!("ledger-{parsed}.sqlite3"))
        })
    }
    fn authenticate(
        &mut self,
        id: &str,
        remember: Option<bool>,
    ) -> Result<ProfileResponse, AppError> {
        let path = self.ledger_path(id)?;
        // Detect storage failures before replacing a usable session.
        drop(SqliteLedger::open(&path)?);
        let flag = Arc::new(AtomicBool::new(true));
        let worker = LedgerWorker::start(path)?.guarded(flag.clone());
        let session = ProfileSession {
            token: Uuid::new_v4().to_string(),
            profile: self.profile(id)?,
        };
        if let Some(remember) = remember {
            let tx = self
                .connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| profile_error())?;
            if remember {
                self.remembered.save(&tx, id, now()?)?;
            } else {
                self.remembered.revoke(&tx)?;
            }
            tx.commit().map_err(|_| profile_error())?;
        }
        let mut active = self.active.lock().map_err(|_| profile_error())?;
        if let Some(previous) = active.take() {
            previous.active.store(false, Ordering::Release);
        }
        *active = Some(ActiveSession {
            session: session.clone(),
            worker,
            active: flag,
        });
        Ok(ProfileResponse::Authenticated(session))
    }
    fn execute(&mut self, command: ProfileCommand) -> Result<ProfileResponse, AppError> {
        match command {
            ProfileCommand::List => Ok(ProfileResponse::Profiles(self.list()?)),
            ProfileCommand::Resume => {
                if self.active.lock().map_err(|_| profile_error())?.is_some() {
                    return Err(login_required());
                }
                let tx = self
                    .connection
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| profile_error())?;
                let remembered = self.remembered.resume(&tx, now()?)?;
                tx.commit().map_err(|_| profile_error())?;
                match remembered {
                    Some(id) => self.authenticate(&id, None),
                    None => Ok(ProfileResponse::SignedOut),
                }
            }
            ProfileCommand::Create {
                username,
                password,
                remember,
            } => {
                let password = Zeroizing::new(password);
                let name = Username::new(&username)?;
                self.unique(&name, None)?;
                if self.list()?.len() >= 100 {
                    return Err(AppError::Input("สร้างผู้ใช้ได้ไม่เกิน 100 คนต่อเครื่อง".into()));
                }
                let hash = hash_password(&password)?;
                let id = Uuid::new_v4().to_string();
                self.connection.execute("INSERT INTO profiles(id,username,name_key,password_hash,legacy) VALUES(?1,?2,?3,?4,0)",params![id,name.as_str(),name.key(),hash]).map_err(|_| profile_error())?;
                self.authenticate(&id, Some(remember))
            }
            ProfileCommand::Login {
                id,
                password,
                remember,
            } => {
                let password = Zeroizing::new(password);
                self.verify(&id, &password)?;
                self.authenticate(&id, Some(remember))
            }
            ProfileCommand::ClaimLegacy {
                id,
                username,
                password,
                remember,
            } => {
                let password = Zeroizing::new(password);
                let name = Username::new(&username)?;
                self.unique(&name, Some(&id))?;
                let hash = hash_password(&password)?;
                let count = self.connection.execute("UPDATE profiles SET username=?1,name_key=?2,password_hash=?3 WHERE id=?4 AND legacy=1 AND password_hash IS NULL", params![name.as_str(),name.key(),hash,id]).map_err(|_| profile_error())?;
                if count != 1 {
                    return Err(login_required());
                }
                self.authenticate(&id, Some(remember))
            }
            ProfileCommand::Edit {
                token,
                username,
                current_password,
                new_password,
            } => {
                let current_password = Zeroizing::new(current_password);
                let new_password = new_password.map(Zeroizing::new);
                let session = self
                    .active
                    .lock()
                    .map_err(|_| profile_error())?
                    .as_ref()
                    .filter(|a| a.session.token == token)
                    .map(|a| a.session.clone())
                    .ok_or_else(login_required)?;
                self.verify(&session.profile.id, &current_password)?;
                let name = Username::new(&username)?;
                self.unique(&name, Some(&session.profile.id))?;
                let hash = new_password
                    .as_ref()
                    .map(|p| hash_password(p))
                    .transpose()?;
                let tx = self
                    .connection
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| profile_error())?;
                tx.execute("UPDATE profiles SET username=?1,name_key=?2,password_hash=COALESCE(?3,password_hash) WHERE id=?4",params![name.as_str(),name.key(),hash,session.profile.id]).map_err(|_| profile_error())?;
                if hash.is_some() {
                    self.remembered.revoke(&tx)?;
                }
                tx.commit().map_err(|_| profile_error())?;
                let updated = self.profile(&session.profile.id)?;
                let mut active = self.active.lock().map_err(|_| profile_error())?;
                let active = active
                    .as_mut()
                    .filter(|a| a.session.token == token)
                    .ok_or_else(login_required)?;
                active.session.profile = updated;
                Ok(ProfileResponse::Authenticated(active.session.clone()))
            }
            ProfileCommand::Logout { token } => {
                let mut active = self.active.lock().map_err(|_| profile_error())?;
                if active.as_ref().is_none_or(|a| a.session.token != token) {
                    return Err(login_required());
                }
                let tx = self
                    .connection
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                    .map_err(|_| profile_error())?;
                self.remembered.revoke(&tx)?;
                tx.commit().map_err(|_| profile_error())?;
                if let Some(previous) = active.take() {
                    previous.active.store(false, Ordering::Release);
                }
                Ok(ProfileResponse::SignedOut)
            }
        }
    }
}

struct Request {
    command: ProfileCommand,
    reply: oneshot::Sender<Result<ProfileResponse, AppError>>,
}
#[derive(Clone)]
pub struct ProfileWorker {
    sender: SyncSender<Request>,
    active: SharedSession,
}
impl ProfileWorker {
    pub fn start(directory: &Path) -> Result<Self, AppError> {
        let directory = directory.to_owned();
        let active = Arc::new(Mutex::new(None));
        let shared = active.clone();
        let (sender, receiver) = mpsc::sync_channel::<Request>(4);
        std::thread::Builder::new()
            .name("profile-authentication".into())
            .spawn(move || {
                let mut registry = Registry::open(directory, shared);
                while let Ok(request) = receiver.recv() {
                    let answer = match &mut registry {
                        Ok(registry) => registry.execute(request.command),
                        Err(error) => Err(error.clone()),
                    };
                    let _ = request.reply.send(answer);
                }
            })
            .map_err(|_| AppError::WorkerStopped)?;
        Ok(Self { sender, active })
    }
    pub fn ledger(&self, token: &str) -> Result<LedgerWorker, AppError> {
        self.active
            .lock()
            .map_err(|_| profile_error())?
            .as_ref()
            .filter(|a| a.session.token == token && a.active.load(Ordering::Acquire))
            .map(|a| a.worker.clone())
            .ok_or_else(login_required)
    }
    pub async fn request(&self, command: ProfileCommand) -> Result<ProfileResponse, AppError> {
        let (reply, receiver) = oneshot::channel();
        self.sender
            .try_send(Request { command, reply })
            .map_err(|e| match e {
                TrySendError::Full(_) => AppError::Busy,
                TrySendError::Disconnected(_) => AppError::WorkerStopped,
            })?;
        receiver.await.map_err(|_| AppError::WorkerStopped)?
    }
    pub fn profile_key(&self, token: &str) -> Result<String, AppError> {
        self.active
            .lock()
            .map_err(|_| profile_error())?
            .as_ref()
            .filter(|a| a.session.token == token && a.active.load(Ordering::Acquire))
            .map(|a| a.session.profile.id.clone())
            .ok_or_else(login_required)
    }
}
