use crate::AppError;

/// Login identity is separate from a financial account. Display casing is kept;
/// uniqueness is checked using the normalized key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Username(String);
impl Username {
    pub fn new(value: &str) -> Result<Self, AppError> {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 40 || value.chars().any(char::is_control) {
            return Err(AppError::Input(
                "ชื่อผู้ใช้ต้องมี 1–40 ตัวอักษร และไม่มีอักขระควบคุม".into(),
            ));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn key(&self) -> String {
        self.0.to_lowercase()
    }
}

pub fn validate_new_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() < 8 || password.len() > 256 || password.trim().is_empty() {
        return Err(AppError::Input(
            "รหัสผ่านต้องมีอย่างน้อย 8 ตัวอักษร และไม่เกิน 256 ไบต์".into(),
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub needs_password: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileSession {
    pub token: String,
    pub profile: UserProfile,
}

/// Fixed lifetime of a remembered login. Reopening the app never extends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RememberedLoginLifetime {
    issued_at: i64,
    expires_at: i64,
}
impl RememberedLoginLifetime {
    const SEVEN_DAYS: i64 = 7 * 24 * 60 * 60;

    pub fn starting_at(issued_at: i64) -> Result<Self, AppError> {
        let expires_at = issued_at
            .checked_add(Self::SEVEN_DAYS)
            .filter(|_| issued_at >= 0)
            .ok_or_else(login_required)?;
        Ok(Self {
            issued_at,
            expires_at,
        })
    }

    pub fn expires_at(self) -> i64 {
        self.expires_at
    }

    pub fn accepts(self, now: i64, last_used_at: i64) -> bool {
        self.issued_at <= last_used_at && last_used_at <= now && now < self.expires_at
    }
}

// Intentionally no Debug: credentials must never enter diagnostic logs.
pub enum ProfileCommand {
    List,
    Resume,
    Create {
        username: String,
        password: String,
        remember: bool,
    },
    Login {
        id: String,
        password: String,
        remember: bool,
    },
    ClaimLegacy {
        id: String,
        username: String,
        password: String,
        remember: bool,
    },
    Edit {
        token: String,
        username: String,
        current_password: String,
        new_password: Option<String>,
    },
    Logout {
        token: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileResponse {
    Profiles(Vec<UserProfile>),
    Authenticated(ProfileSession),
    SignedOut,
}
pub fn login_required() -> AppError {
    AppError::Input("กรุณาเข้าสู่ระบบอีกครั้ง".into())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn remembered_login_has_a_fixed_seven_day_boundary_and_rejects_clock_rollback() {
        let lifetime = RememberedLoginLifetime::starting_at(1_000).expect("valid time");
        assert_eq!(lifetime.expires_at(), 605_800);
        assert!(lifetime.accepts(1_000, 1_000));
        assert!(lifetime.accepts(605_799, 500_000));
        assert!(!lifetime.accepts(605_800, 500_000));
        assert!(!lifetime.accepts(605_801, 500_000));
        assert!(!lifetime.accepts(999, 1_000));
        assert!(!lifetime.accepts(2_000, 2_001));
        assert!(!lifetime.accepts(2_000, 999));
        assert!(RememberedLoginLifetime::starting_at(-1).is_err());
        assert!(RememberedLoginLifetime::starting_at(i64::MAX).is_err());
    }
}
