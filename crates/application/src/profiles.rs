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

// Intentionally no Debug: credentials must never enter diagnostic logs.
pub enum ProfileCommand {
    List,
    Create {
        username: String,
        password: String,
    },
    Login {
        id: String,
        password: String,
    },
    ClaimLegacy {
        id: String,
        username: String,
        password: String,
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
