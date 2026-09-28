//! Only the fixed public release repository can supply executable updates.
use self::ErrorKind::*;
use ledger_application::{AppError, AppRelease, UpdateCheck, UpdateOperation, UpdatePlatform};
use reqwest::{blocking::Client, header};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::Ordering},
    time::{Duration, Instant},
};

const API: &str = "https://api.github.com/repos/Rayato159/ledgers-bro/releases/latest";
const REPO: &str = "https://github.com/Rayato159/ledgers-bro";
const MAX_INSTALLER: u64 = 1024 * 1024 * 1024;
#[derive(Clone, Copy)]
enum ErrorKind {
    Network,
    Invalid,
    Cancelled,
    Storage,
    NotReady,
}
fn error(kind: ErrorKind) -> AppError {
    AppError::Input(
        match kind {
            Network => "ตรวจหรือดาวน์โหลดอัปเดตไม่สำเร็จ ตรวจอินเทอร์เน็ตแล้วลองใหม่",
            Invalid => "ไฟล์อัปเดตหรือข้อมูลรุ่นไม่ผ่านการตรวจสอบ ยังไม่ได้ติดตั้ง",
            Cancelled => "ยกเลิกการดาวน์โหลดอัปเดตแล้ว",
            Storage => "พื้นที่ว่างไม่พอหรือเขียนไฟล์อัปเดตไม่ได้",
            NotReady => "กรุณาตรวจและดาวน์โหลดอัปเดตก่อนติดตั้ง",
        }
        .into(),
    )
}
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubAsset>,
}
#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    state: String,
    digest: Option<String>,
    browser_download_url: String,
}
fn version(text: &str) -> Result<semver::Version, AppError> {
    let v = semver::Version::parse(text).map_err(|_| error(Invalid))?;
    if !v.pre.is_empty() || !v.build.is_empty() {
        return Err(error(Invalid));
    }
    Ok(v)
}
fn parse_release(
    bytes: &[u8],
    current: &str,
    platform: UpdatePlatform,
) -> Result<UpdateCheck, AppError> {
    let release: GithubRelease = serde_json::from_slice(bytes).map_err(|_| error(Invalid))?;
    let tag = release
        .tag_name
        .strip_prefix('v')
        .ok_or_else(|| error(Invalid))?;
    let latest = version(tag)?;
    if release.draft
        || release.prerelease
        || release.html_url != format!("{REPO}/releases/tag/v{latest}")
    {
        return Err(error(Invalid));
    }
    let mut result = UpdateCheck {
        current: current.into(),
        available: None,
    };
    if latest <= version(current)? {
        return Ok(result);
    }
    let filename = platform.asset_name(&latest.to_string());
    let mut assets = release.assets.into_iter().filter(|a| a.name == filename);
    let asset = assets.next().ok_or_else(|| error(Invalid))?;
    if assets.next().is_some() {
        return Err(error(Invalid));
    }
    let hash = asset
        .digest
        .and_then(|s| s.strip_prefix("sha256:").map(str::to_owned))
        .ok_or_else(|| error(Invalid))?;
    if asset.state != "uploaded"
        || asset.size == 0
        || asset.size > MAX_INSTALLER
        || hash.len() != 64
        || !hash.bytes().all(|c| c.is_ascii_hexdigit())
        || asset.browser_download_url != format!("{REPO}/releases/download/v{latest}/{filename}")
    {
        return Err(error(Invalid));
    }
    result.available = Some(AppRelease {
        version: latest.to_string(),
        page: release.html_url,
        filename,
        bytes: asset.size,
        sha256: hash.to_ascii_lowercase(),
        download_url: asset.browser_download_url,
    });
    Ok(result)
}
fn client() -> Result<Client, AppError> {
    Client::builder()
        .user_agent("LedgersBro-Updater")
        .https_only(true)
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("Too many redirects");
            }
            if attempt.url().scheme() == "https"
                && matches!(
                    attempt.url().host_str(),
                    Some(
                        "github.com"
                            | "api.github.com"
                            | "release-assets.githubusercontent.com"
                            | "objects.githubusercontent.com"
                    )
                )
            {
                attempt.follow()
            } else {
                attempt.error("Unexpected update host")
            }
        }))
        .build()
        .map_err(|_| error(Network))
}
fn cancel(op: &UpdateOperation) -> Result<(), AppError> {
    if op.cancelled.load(Ordering::Relaxed) {
        Err(error(Cancelled))
    } else {
        Ok(())
    }
}
fn verify(path: &Path, release: &AppRelease, op: &UpdateOperation) -> Result<(), AppError> {
    let mut file = File::open(path).map_err(|_| error(NotReady))?;
    if file.metadata().map_err(|_| error(Invalid))?.len() != release.bytes {
        return Err(error(Invalid));
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        cancel(op)?;
        let n = file.read(&mut buffer).map_err(|_| error(Invalid))?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    if format!("{:x}", hash.finalize()) != release.sha256 {
        return Err(error(Invalid));
    }
    Ok(())
}

#[derive(Default)]
struct UpdateState {
    checked: Option<AppRelease>,
    ready: Option<AppRelease>,
}
#[derive(Clone)]
pub struct AppUpdater {
    directory: PathBuf,
    current: String,
    platform: UpdatePlatform,
    state: Arc<Mutex<UpdateState>>,
}
impl AppUpdater {
    pub fn new(directory: PathBuf, current: String, platform: UpdatePlatform) -> Self {
        let updater = Self {
            directory,
            current,
            platform,
            state: Arc::new(Mutex::new(UpdateState::default())),
        };
        // Only the running version proves that an update completed. Never delete
        // a newer package while Windows/Android may still be reading it.
        updater.cleanup_installed_packages();
        updater
    }

    fn cleanup_installed_packages(&self) {
        let Ok(current) = version(&self.current) else {
            return;
        };
        let Ok(metadata) = std::fs::symlink_metadata(&self.directory) else {
            return;
        };
        if !metadata.is_dir() || is_link(&metadata) {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&self.directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(installed) = path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(package_version)
            else {
                continue;
            };
            if installed > current {
                continue;
            }
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.is_file() && !is_link(&metadata) {
                // Locked packages are retried next launch/check; cleanup failure
                // must never block login or an otherwise valid update.
                let _ = std::fs::remove_file(path);
            }
        }
    }
    pub async fn check(&self) -> Result<UpdateCheck, AppError> {
        let this = self.clone();
        background(move || {
            this.cleanup_installed_packages();
            let response = client()?
                .get(API)
                .header(header::ACCEPT, "application/vnd.github+json")
                .send()
                .map_err(|_| error(Network))?;
            if !response.status().is_success() {
                return Err(error(Network));
            }
            let mut bytes = Vec::new();
            response
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| error(Network))?;
            if bytes.len() > 1024 * 1024 {
                return Err(error(Invalid));
            }
            let result = parse_release(&bytes, &this.current, this.platform)?;
            let mut state = this.state.lock().map_err(|_| error(NotReady))?;
            if state.checked != result.available {
                state.ready = None;
            }
            state.checked = result.available.clone();
            Ok(result)
        })
        .await
    }
    pub async fn download(
        &self,
        release: AppRelease,
        operation: UpdateOperation,
    ) -> Result<(), AppError> {
        let this = self.clone();
        background(move || {
            // One active update per app, even if different profiles have open views.
            let mut state = this.state.try_lock().map_err(|_| error(NotReady))?;
            if state.checked.as_ref() != Some(&release) {
                return Err(error(NotReady));
            }
            cancel(&operation)?;
            std::fs::create_dir_all(&this.directory).map_err(|_| error(Storage))?;
            this.cleanup_installed_packages();
            let path = this.directory.join(&release.filename);
            if verify(&path, &release, &operation).is_ok() {
                state.ready = Some(release);
                return Ok(());
            }
            cancel(&operation)?;
            if fs2::available_space(&this.directory).map_err(|_| error(Storage))?
                < release.bytes + 64 * 1024 * 1024
            {
                return Err(error(Storage));
            }
            let mut temporary =
                tempfile::NamedTempFile::new_in(&this.directory).map_err(|_| error(Storage))?;
            let client = client()?;
            let started = Instant::now();
            let mut downloaded = 0;
            let mut buffer = [0; 65536];
            while downloaded < release.bytes {
                cancel(&operation)?;
                if started.elapsed() > Duration::from_secs(7200) {
                    return Err(error(Network));
                }
                let end = (downloaded + 8 * 1024 * 1024).min(release.bytes) - 1;
                let mut response = client
                    .get(&release.download_url)
                    .header(header::RANGE, format!("bytes={downloaded}-{end}"))
                    .send()
                    .map_err(|_| error(Network))?;
                let expected = format!("bytes {downloaded}-{end}/{}", release.bytes);
                if response.status() != reqwest::StatusCode::PARTIAL_CONTENT
                    || response
                        .headers()
                        .get(header::CONTENT_RANGE)
                        .and_then(|s| s.to_str().ok())
                        != Some(expected.as_str())
                {
                    return Err(error(Invalid));
                }
                loop {
                    cancel(&operation)?;
                    let count = response.read(&mut buffer).map_err(|_| error(Network))?;
                    if count == 0 {
                        break;
                    }
                    downloaded += count as u64;
                    if downloaded > end + 1 {
                        return Err(error(Invalid));
                    }
                    temporary
                        .write_all(&buffer[..count])
                        .map_err(|_| error(Storage))?;
                    operation.downloaded.store(downloaded, Ordering::Relaxed);
                }
                if downloaded != end + 1 {
                    return Err(error(Invalid));
                }
            }
            temporary.as_file().sync_all().map_err(|_| error(Storage))?;
            verify(temporary.path(), &release, &operation)?;
            cancel(&operation)?;
            temporary.persist(path).map_err(|_| error(Storage))?;
            state.ready = Some(release);
            Ok(())
        })
        .await
    }
    /// Revalidate cached bytes immediately before handing the installer to the OS.
    pub async fn installer(&self) -> Result<(PathBuf, AppRelease), AppError> {
        let this = self.clone();
        background(move || {
            let state = this.state.lock().map_err(|_| error(NotReady))?;
            let release = state.ready.clone().ok_or_else(|| error(NotReady))?;
            let path = this.directory.join(&release.filename);
            verify(&path, &release, &UpdateOperation::default())?;
            Ok((path, release))
        })
        .await
    }
}

/// Match only filenames produced by our updater, in its own flat cache folder.
fn package_version(name: &str) -> Option<semver::Version> {
    let rest = name.strip_prefix("LedgersBro-")?;
    let text = [
        "-windows-x64.msi",
        "-android-arm64-test.apk",
        "-android-x86_64-test.apk",
    ]
    .into_iter()
    .find_map(|suffix| rest.strip_suffix(suffix))?;
    let parsed = version(text).ok()?;
    (parsed.to_string() == text).then_some(parsed)
}

fn is_link(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0 // FILE_ATTRIBUTE_REPARSE_POINT
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

pub(crate) async fn background<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let (tx, rx) = futures_channel::oneshot::channel();
    std::thread::Builder::new()
        .name("app-updater".into())
        .spawn(move || {
            let _ = tx.send(f());
        })
        .map_err(|_| AppError::WorkerStopped)?;
    rx.await.map_err(|_| AppError::WorkerStopped)?
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    fn fixture(tag: &str) -> serde_json::Value {
        let v = tag.trim_start_matches('v');
        let name = UpdatePlatform::WindowsX64.asset_name(v);
        serde_json::json!({"tag_name":tag,"html_url":format!("{REPO}/releases/tag/{tag}"),"draft":false,"prerelease":false,"assets":[{"name":name,"state":"uploaded","size":42,"digest":format!("sha256:{}","a".repeat(64)),"browser_download_url":format!("{REPO}/releases/download/{tag}/{name}")}]})
    }
    fn parse(v: serde_json::Value, current: &str) -> Result<UpdateCheck, AppError> {
        parse_release(
            v.to_string().as_bytes(),
            current,
            UpdatePlatform::WindowsX64,
        )
    }
    #[test]
    fn versions_are_numeric_and_never_downgrade() {
        assert!(parse(fixture("v0.1.10"), "0.1.9").is_ok_and(|s| s.available.is_some()));
        for tag in ["v0.1.4", "v0.1.3"] {
            assert!(parse(fixture(tag), "0.1.4").is_ok_and(|s| s.available.is_none()));
        }
        assert!(parse(fixture("v0.2.0-rc.1"), "0.1.4").is_err());
    }
    #[test]
    fn wrong_assets_redirects_and_unverified_files_are_rejected() {
        for field in ["digest", "state", "browser_download_url", "name"] {
            let mut f = fixture("v0.1.5");
            f["assets"][0][field] = serde_json::json!("unexpected");
            assert!(parse(f, "0.1.4").is_err());
        }
        let mut f = fixture("v0.1.5");
        f["prerelease"] = true.into();
        assert!(parse(f, "0.1.4").is_err());
        let mut f = fixture("v0.1.5");
        let a = f["assets"][0].clone();
        if let Some(list) = f["assets"].as_array_mut() {
            list.push(a);
        }
        assert!(parse(f, "0.1.4").is_err());
        assert!(
            parse_release(
                fixture("v0.1.5").to_string().as_bytes(),
                "0.1.4",
                UpdatePlatform::AndroidArm64
            )
            .is_err()
        );
    }

    #[test]
    fn installer_rechecks_cached_bytes_and_cancellation_never_marks_ready() {
        let directory = tempfile::tempdir().expect("test directory");
        let updater = AppUpdater::new(
            directory.path().into(),
            "0.1.4".into(),
            UpdatePlatform::WindowsX64,
        );
        let content = b"synthetic installer bytes only";
        let mut release = parse(fixture("v0.1.5"), "0.1.4")
            .expect("release")
            .available
            .expect("new version");
        release.bytes = content.len() as u64;
        release.sha256 = format!("{:x}", Sha256::digest(content));
        updater.state.lock().expect("state").checked = Some(release.clone());
        let path = directory.path().join(&release.filename);
        std::fs::write(&path, content).expect("cache");
        let cancelled = UpdateOperation::default();
        cancelled.cancelled.store(true, Ordering::Relaxed);
        assert!(futures_executor::block_on(updater.download(release.clone(), cancelled)).is_err());
        assert!(futures_executor::block_on(updater.installer()).is_err());
        futures_executor::block_on(updater.download(release, UpdateOperation::default()))
            .expect("verified cache");
        assert_eq!(
            futures_executor::block_on(updater.installer())
                .expect("ready")
                .0,
            path
        );
        let mut corrupted = *content;
        corrupted[0] ^= 1;
        std::fs::write(&path, corrupted).expect("same-size corruption");
        assert!(futures_executor::block_on(updater.installer()).is_err());
        assert_eq!(
            std::fs::read_dir(directory.path()).expect("files").count(),
            1
        );
    }

    #[test]
    fn startup_cleans_only_installed_packages_in_the_update_cache() {
        for platform in [
            UpdatePlatform::WindowsX64,
            UpdatePlatform::AndroidArm64,
            UpdatePlatform::AndroidX64,
        ] {
            let directory = tempfile::tempdir().expect("isolated cache");
            let cache = directory.path().join("updates");
            std::fs::create_dir(&cache).expect("cache");
            let old = platform.asset_name("0.1.9");
            let current = platform.asset_name("0.1.12");
            let newer = platform.asset_name("0.1.13");
            let preserved = [
                newer.as_str(),
                "ledger.sqlite3",
                "model.gguf",
                "personal.apk",
                "LedgersBro-0.1.12-windows-x64.zip",
                "LedgersBro-0.1.12-android-arm64-test.apk.backup",
                "LedgersBro-0.1.12-beta-windows-x64.msi",
                "LedgersBro-00.1.12-windows-x64.msi",
            ];
            for name in [old.as_str(), current.as_str()]
                .into_iter()
                .chain(preserved)
            {
                std::fs::write(cache.join(name), b"synthetic cache fixture").expect("fixture");
            }
            let nested = cache.join("LedgersBro-0.0.1-windows-x64.msi");
            std::fs::create_dir(&nested).expect("directory must not be removed");
            std::fs::write(nested.join(&old), b"nested fixture").expect("nested file");
            std::fs::write(directory.path().join(&old), b"outside cache").expect("outside file");
            let _ = AppUpdater::new(cache.clone(), "0.1.12".into(), platform);
            assert!(!cache.join(old.as_str()).exists() && !cache.join(current).exists());
            for name in preserved {
                assert!(cache.join(name).is_file(), "preserve {name}");
            }
            assert!(nested.join(&old).is_file());
            assert!(directory.path().join(old).is_file());
        }
    }

    #[test]
    fn pending_install_survives_until_the_new_version_starts() {
        let directory = tempfile::tempdir().expect("isolated cache");
        let platform = UpdatePlatform::WindowsX64;
        let path = directory.path().join(platform.asset_name("0.1.12"));
        std::fs::write(&path, b"synthetic pending installer").expect("fixture");
        let old = AppUpdater::new(directory.path().into(), "0.1.11".into(), platform);
        old.cleanup_installed_packages();
        assert!(
            path.is_file(),
            "opening or cancelling installation is not success"
        );
        let _ = AppUpdater::new(directory.path().into(), "0.1.12".into(), platform);
        assert!(!path.exists());
    }

    #[test]
    fn cleanup_does_not_create_folders_or_guess_invalid_versions() {
        let directory = tempfile::tempdir().expect("isolated cache");
        let missing = directory.path().join("missing");
        let platform = UpdatePlatform::WindowsX64;
        let _ = AppUpdater::new(missing.clone(), "0.1.12".into(), platform);
        assert!(!missing.exists());
        let path = directory.path().join(platform.asset_name("0.1.11"));
        std::fs::write(&path, b"synthetic fixture").expect("fixture");
        let _ = AppUpdater::new(directory.path().into(), "invalid".into(), platform);
        assert!(path.is_file());
    }

    #[cfg(windows)]
    #[test]
    fn locked_installer_is_preserved_and_retried_after_release() {
        use std::os::windows::fs::OpenOptionsExt;
        let directory = tempfile::tempdir().expect("isolated cache");
        let path = directory
            .path()
            .join(UpdatePlatform::WindowsX64.asset_name("0.1.12"));
        std::fs::write(&path, b"synthetic locked installer").expect("fixture");
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .expect("locked file");
        let updater = AppUpdater::new(
            directory.path().into(),
            "0.1.12".into(),
            UpdatePlatform::WindowsX64,
        );
        assert!(path.is_file());
        drop(locked);
        updater.cleanup_installed_packages();
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn cleanup_never_follows_cache_or_file_links() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().expect("isolated cache");
        let cache = directory.path().join("updates");
        let outside = directory.path().join("outside");
        std::fs::create_dir(&outside).expect("outside");
        let name = UpdatePlatform::AndroidArm64.asset_name("0.1.12");
        std::fs::write(outside.join(&name), b"outside fixture").expect("fixture");
        symlink(&outside, &cache).expect("directory link");
        let _ = AppUpdater::new(cache.clone(), "0.1.12".into(), UpdatePlatform::AndroidArm64);
        assert!(outside.join(&name).is_file());
        std::fs::remove_file(&cache).expect("remove test link");
        std::fs::create_dir(&cache).expect("cache");
        symlink(outside.join(&name), cache.join(&name)).expect("file link");
        let _ = AppUpdater::new(cache.clone(), "0.1.12".into(), UpdatePlatform::AndroidArm64);
        assert!(
            cache
                .join(&name)
                .symlink_metadata()
                .expect("link")
                .file_type()
                .is_symlink()
        );
        assert!(outside.join(&name).is_file());
    }

    #[test]
    #[ignore = "downloads public release installers; run explicitly with network access"]
    fn live_release_downloads_match_platform_and_hash() {
        for platform in [UpdatePlatform::WindowsX64, UpdatePlatform::AndroidX64] {
            let directory = tempfile::tempdir().expect("isolated download directory");
            let updater = AppUpdater::new(directory.path().into(), "0.0.0".into(), platform);
            let release = futures_executor::block_on(updater.check())
                .expect("GitHub check")
                .available
                .expect("published release");
            assert_eq!(release.filename, platform.asset_name(&release.version));
            futures_executor::block_on(
                updater.download(release.clone(), UpdateOperation::default()),
            )
            .expect("download and SHA-256");
            let (path, ready) =
                futures_executor::block_on(updater.installer()).expect("verified installer");
            assert_eq!(ready, release);
            assert_eq!(
                std::fs::metadata(path).expect("download").len(),
                release.bytes
            );
            // Do not launch an installer or open any user's ledger.
        }
    }
}
