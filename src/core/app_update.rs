use crate::core::http::get_bytes;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RELEASE_URL: &str = "https://api.github.com/repos/anthod0/singBoost/releases/latest";
const ASSET: &str = "singboost-windows-x86_64.exe";
const MAX_EXE_SIZE: u64 = 100 * 1024 * 1024;
pub const STAGING_PREFIX: &str = ".singboost-update-";
type UpdateResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct AppUpdate {
    pub version: Version,
    url: String,
    size: u64,
    digest: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

impl AppUpdate {
    pub fn from_release_json(body: &[u8]) -> UpdateResult<Self> {
        let release: Release = serde_json::from_slice(body)?;
        if release.draft || release.prerelease {
            return Err("release is not a stable release".into());
        }
        let version = Version::parse(release.tag_name.trim_start_matches('v'))?;
        if !version.pre.is_empty() {
            return Err("release version is a prerelease".into());
        }
        let asset = release
            .assets
            .into_iter()
            .find(|a| a.name == ASSET)
            .ok_or("release has no supported Windows executable")?;
        if asset.size == 0
            || asset.size > MAX_EXE_SIZE
            || !asset
                .browser_download_url
                .starts_with("https://github.com/anthod0/singBoost/releases/download/")
        {
            return Err("invalid release asset".into());
        }
        let digest = asset
            .digest
            .and_then(|d| d.strip_prefix("sha256:").map(str::to_owned))
            .filter(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or("release asset has no valid SHA-256 digest")?;
        Ok(Self {
            version,
            url: asset.browser_download_url,
            size: asset.size,
            digest,
        })
    }

    pub fn available(&self) -> bool {
        self.version > Version::parse(env!("CARGO_PKG_VERSION")).expect("package version")
    }

    pub fn validate_download(&self, bytes: &[u8]) -> UpdateResult<()> {
        if bytes.len() as u64 != self.size {
            return Err("download size mismatch".into());
        }
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if digest != self.digest.to_ascii_lowercase() {
            return Err("download SHA-256 mismatch".into());
        }
        validate_executable(bytes)
    }
}

pub fn validate_executable(bytes: &[u8]) -> UpdateResult<()> {
    let offset = bytes.get(60..64).ok_or("truncated DOS header")?;
    let offset = u32::from_le_bytes(offset.try_into()?) as usize;
    let header = bytes
        .get(offset..offset.checked_add(26).ok_or("invalid PE offset")?)
        .ok_or("truncated PE header")?;
    if bytes.get(..2) != Some(b"MZ")
        || &header[..4] != b"PE\0\0"
        || u16::from_le_bytes(header[4..6].try_into()?) != 0x8664
        || u16::from_le_bytes(header[22..24].try_into()?) & 0x2002 != 0x0002
        || u16::from_le_bytes(header[24..26].try_into()?) != 0x20b
    {
        return Err("asset is not an x86_64 Windows executable".into());
    }
    Ok(())
}

pub fn check_app_update() -> UpdateResult<AppUpdate> {
    if !cfg!(target_arch = "x86_64") {
        return Err("SingBoost updates only support x86_64 Windows".into());
    }
    AppUpdate::from_release_json(&get_bytes(
        RELEASE_URL,
        Duration::from_secs(30),
        2 * 1024 * 1024,
    )?)
}

#[derive(Debug)]
pub struct PreparedAppUpdate {
    pub directory: PathBuf,
    handed_off: bool,
}
impl PreparedAppUpdate {
    pub fn hand_off(&mut self) {
        self.handed_off = true;
    }
}
impl Drop for PreparedAppUpdate {
    fn drop(&mut self) {
        if !self.handed_off {
            cleanup_update_directory(&self.directory);
        }
    }
}

pub fn prepare_app_update(app_dir: &Path, update: &AppUpdate) -> UpdateResult<PreparedAppUpdate> {
    let bytes = get_bytes(&update.url, Duration::from_secs(300), MAX_EXE_SIZE)?;
    update.validate_download(&bytes)?;
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let directory = app_dir.join(format!("{STAGING_PREFIX}{}-{nonce}", std::process::id()));
    std::fs::create_dir(&directory)?;
    let prepared = PreparedAppUpdate {
        directory,
        handed_off: false,
    };
    std::fs::write(prepared.directory.join("new.exe"), &bytes)?;
    std::fs::write(prepared.directory.join("helper.exe"), &bytes)?;
    Ok(prepared)
}

// Never recursively remove a path supplied through command-line arguments.
pub fn cleanup_update_directory(directory: &Path) {
    for name in [
        "new.exe",
        "helper.exe",
        "ready",
        "ready.tmp",
        "started",
        "started.tmp",
        "old.exe",
    ] {
        let _ = std::fs::remove_file(directory.join(name));
    }
    let _ = std::fs::remove_dir(directory);
}

/// Replace the executable, restoring it if replacement or launch fails.
/// The caller must first wait for the old process to exit.
pub fn install_app_update(
    target: &Path,
    directory: &Path,
    launch: impl Fn(&Path) -> UpdateResult<()>,
) -> UpdateResult<()> {
    let replacement = directory.join("new.exe");
    validate_executable(&std::fs::read(&replacement)?)?;
    let backup = directory.join("old.exe");
    if backup.exists() {
        return Err("update backup already exists".into());
    }
    std::fs::rename(target, &backup)?;
    let result = std::fs::rename(&replacement, target)
        .map_err(Into::into)
        .and_then(|()| launch(target));
    if let Err(error) = result {
        if target.exists() {
            std::fs::rename(target, &replacement)?;
        }
        std::fs::rename(&backup, target).map_err(|rollback| {
            format!(
                "{error}; rollback failed: {rollback}; backup: {}",
                backup.display()
            )
        })?;
        launch(target).map_err(|restart| {
            format!("{error}; old version restored but restart failed: {restart}")
        })?;
        return Err(error);
    }
    Ok(())
}
