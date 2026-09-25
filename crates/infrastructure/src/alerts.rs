use ledger_application::{AlertPreferences, AppError};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct AlertStore {
    path: PathBuf,
}
impl AlertStore {
    pub fn new(directory: &Path, profile: &str) -> Result<Self, AppError> {
        let id =
            uuid::Uuid::parse_str(profile).map_err(|_| ledger_application::login_required())?;
        Ok(Self {
            path: directory.join(format!("notifications-{id}.json")),
        })
    }
    pub async fn load(&self) -> Result<AlertPreferences, AppError> {
        let path = self.path.clone();
        crate::update::background(move || {
            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(AlertPreferences::default());
                }
                Err(_) => return Err(error()),
            };
            let mut bytes = Vec::new();
            file.take(1048577)
                .read_to_end(&mut bytes)
                .map_err(|_| error())?;
            if bytes.len() > 1048576 {
                return Err(error());
            }
            let prefs: AlertPreferences = serde_json::from_slice(&bytes).map_err(|_| error())?;
            if !prefs.valid() {
                return Err(error());
            }
            Ok(prefs)
        })
        .await
    }
    pub async fn save(&self, prefs: AlertPreferences) -> Result<(), AppError> {
        let path = self.path.clone();
        crate::update::background(move || {
            if !prefs.valid() {
                return Err(error());
            }
            let bytes = serde_json::to_vec(&prefs).map_err(|_| error())?;
            if bytes.len() > 1048576 {
                return Err(error());
            }
            let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(error)?)
                .map_err(|_| error())?;
            file.write_all(&bytes).map_err(|_| error())?;
            file.as_file().sync_all().map_err(|_| error())?;
            file.persist(path).map_err(|_| error())?;
            Ok(())
        })
        .await
    }
}
fn error() -> AppError {
    AppError::Input("อ่านหรือบันทึกการแจ้งเตือนไม่สำเร็จ".into())
}
