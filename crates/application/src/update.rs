//! App updates have no access to ledger commands or financial data.
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64},
};

pub const RELEASE_PAGE: &str = "https://github.com/Rayato159/ledgers-bro/releases/latest";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdatePlatform {
    WindowsX64,
    AndroidArm64,
    AndroidX64,
}
impl UpdatePlatform {
    pub fn asset_name(self, version: &str) -> String {
        let suffix = match self {
            Self::WindowsX64 => "windows-x64.msi",
            Self::AndroidArm64 => "android-arm64-test.apk",
            Self::AndroidX64 => "android-x86_64-test.apk",
        };
        format!("LedgersBro-{version}-{suffix}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppRelease {
    pub version: String,
    pub page: String,
    pub filename: String,
    pub bytes: u64,
    pub sha256: String,
    pub download_url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateCheck {
    pub current: String,
    pub available: Option<AppRelease>,
}

#[derive(Clone, Default)]
pub struct UpdateOperation {
    pub cancelled: Arc<AtomicBool>,
    pub downloaded: Arc<AtomicU64>,
}
impl PartialEq for UpdateOperation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cancelled, &other.cancelled)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateInstall {
    /// Android requires the user to allow this app as an installation source.
    PermissionRequired,
    InstallerOpened,
}
