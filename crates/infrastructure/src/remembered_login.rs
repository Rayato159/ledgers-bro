//! Native remembered-login storage, separate from ledger data and WebView storage.
use ledger_application::{AppError, RememberedLoginLifetime};
use rand_core::{OsRng, RngCore};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

const TOKEN_BYTES: usize = 32;
const FILE_NAME: &str = "remembered-login.token";

fn storage_error() -> AppError {
    AppError::Input("บันทึกการจดจำผู้ใช้ไม่สำเร็จ กรุณาลองอีกครั้ง".into())
}

pub(crate) struct RememberedLoginStore {
    directory: PathBuf,
}

impl RememberedLoginStore {
    pub(crate) fn new(directory: &Path) -> Self {
        Self {
            directory: directory.to_owned(),
        }
    }

    pub(crate) fn revoke(&self, connection: &Connection) -> Result<(), AppError> {
        connection
            .execute("DELETE FROM remembered_login", [])
            .map_err(|_| storage_error())?;
        // The caller commits revocation before reporting success. File cleanup
        // is best effort: a leftover credential cannot match a revoked grant.
        let _ = std::fs::remove_file(self.directory.join(FILE_NAME));
        Ok(())
    }

    pub(crate) fn save(
        &self,
        connection: &Connection,
        profile: &str,
        now: i64,
    ) -> Result<(), AppError> {
        let lifetime = RememberedLoginLifetime::starting_at(now)?;
        let mut token = Zeroizing::new([0u8; TOKEN_BYTES]);
        OsRng
            .try_fill_bytes(token.as_mut())
            .map_err(|_| storage_error())?;
        self.revoke(connection)?;
        // NamedTempFile is created with owner-only mode on Unix (including
        // Android). Windows inherits the user's app-data directory permissions.
        let mut file =
            tempfile::NamedTempFile::new_in(&self.directory).map_err(|_| storage_error())?;
        file.write_all(token.as_ref())
            .map_err(|_| storage_error())?;
        file.as_file().sync_all().map_err(|_| storage_error())?;
        file.persist(self.directory.join(FILE_NAME))
            .map_err(|_| storage_error())?;
        let hash = Sha256::digest(token.as_ref());
        connection.execute(
            "INSERT INTO remembered_login(singleton,profile_id,token_hash,issued_at,expires_at,last_used_at) VALUES(1,?1,?2,?3,?4,?3)",
            params![profile, hash.as_slice(), now, lifetime.expires_at()],
        ).map_err(|_| storage_error())?;
        Ok(())
    }

    pub(crate) fn resume(
        &self,
        connection: &Connection,
        now: i64,
    ) -> Result<Option<String>, AppError> {
        let mut file = match File::open(self.directory.join(FILE_NAME)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.revoke(connection)?;
                return Ok(None);
            }
            Err(_) => return Err(storage_error()),
        };
        let mut token = Zeroizing::new([0u8; TOKEN_BYTES]);
        let mut excess = [0u8; 1];
        let valid_length = match file.read_exact(token.as_mut()) {
            Ok(()) => file.read(&mut excess).map_err(|_| storage_error())? == 0,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => false,
            Err(_) => return Err(storage_error()),
        };
        drop(file);
        if !valid_length {
            self.revoke(connection)?;
            return Ok(None);
        }
        let hash = Sha256::digest(token.as_ref());
        let saved: Option<(String, i64, i64, i64)> = connection.query_row(
            "SELECT r.profile_id,r.issued_at,r.expires_at,r.last_used_at FROM remembered_login r JOIN profiles p ON p.id=r.profile_id WHERE r.token_hash=?1 AND p.password_hash IS NOT NULL",
            [hash.as_slice()], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
        ).optional().map_err(|_| storage_error())?;
        if let Some((profile, issued, expires, last_used)) = saved
            && RememberedLoginLifetime::starting_at(issued).is_ok_and(|lifetime| {
                lifetime.expires_at() == expires && lifetime.accepts(now, last_used)
            })
        {
            connection.execute("UPDATE remembered_login SET last_used_at=?1 WHERE singleton=1 AND token_hash=?2", params![now,hash.as_slice()]).map_err(|_| storage_error())?;
            return Ok(Some(profile));
        }
        self.revoke(connection)?;
        Ok(None)
    }
}
