use crate::core::http::get_bytes;
use crate::core::paths::AppPaths;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const LATEST_RELEASE_URL: &str = "https://api.github.com/repos/SagerNet/sing-box/releases/latest";
const RELEASE_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const VERSION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_RELEASE_METADATA_SIZE: u64 = 2 * 1024 * 1024;
const MAX_ARCHIVE_SIZE: u64 = 100 * 1024 * 1024;
const MAX_EXTRACTED_FILE_SIZE: u64 = 150 * 1024 * 1024;
const RUNTIME_FILES: [&str; 2] = ["sing-box.exe", "libcronet.dll"];

#[derive(Debug, Error)]
pub enum KernelUpdateError {
    #[error("unsupported Windows architecture: {0}")]
    UnsupportedArchitecture(String),
    #[error("failed to download release information: {0}")]
    ReleaseDownload(String),
    #[error("invalid release information: {0}")]
    InvalidRelease(String),
    #[error("failed to download sing-box: {0}")]
    Download(String),
    #[error("downloaded archive size does not match release information")]
    SizeMismatch,
    #[error("downloaded archive SHA-256 does not match release information")]
    DigestMismatch,
    #[error("failed to read sing-box archive: {0}")]
    Archive(String),
    #[error("failed to stage sing-box executable: {0}")]
    Stage(#[source] std::io::Error),
    #[error("downloaded sing-box executable is invalid: {0}")]
    InvalidExecutable(String),
    #[error("failed to install sing-box: {message}")]
    Install {
        message: String,
        rollback_failed: bool,
    },
}

#[derive(Debug, Clone)]
pub struct KernelUpdate {
    pub installed_version: Option<Version>,
    pub installed_version_error: Option<String>,
    pub runtime_files_complete: bool,
    pub release_version: Version,
    asset_name: String,
    asset_url: String,
    asset_size: u64,
    sha256: [u8; 32],
}

impl KernelUpdateError {
    pub fn rollback_failed(&self) -> bool {
        matches!(
            self,
            Self::Install {
                rollback_failed: true,
                ..
            }
        )
    }
}

impl KernelUpdate {
    pub fn update_available(&self) -> bool {
        match &self.installed_version {
            None => true,
            Some(installed) if installed < &self.release_version => true,
            Some(installed) if installed == &self.release_version => !self.runtime_files_complete,
            Some(_) => false,
        }
    }
}

#[derive(Debug)]
pub struct PreparedKernelUpdate {
    pub version: Version,
    staging_directory: PathBuf,
}

impl Drop for PreparedKernelUpdate {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.staging_directory);
    }
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

pub fn check_kernel_update(paths: &AppPaths) -> Result<KernelUpdate, KernelUpdateError> {
    let architecture = release_architecture()?;
    let body = get_bytes(
        LATEST_RELEASE_URL,
        RELEASE_TIMEOUT,
        MAX_RELEASE_METADATA_SIZE,
    )
    .map_err(|err| KernelUpdateError::ReleaseDownload(err.to_string()))?;
    let release: GitHubRelease = serde_json::from_slice(&body)
        .map_err(|err| KernelUpdateError::InvalidRelease(err.to_string()))?;
    parse_release(paths, release, architecture)
}

pub fn prepare_kernel_update(
    paths: &AppPaths,
    update: &KernelUpdate,
) -> Result<PreparedKernelUpdate, KernelUpdateError> {
    let archive = get_bytes(&update.asset_url, DOWNLOAD_TIMEOUT, update.asset_size)
        .map_err(|err| KernelUpdateError::Download(err.to_string()))?;
    if archive.len() as u64 != update.asset_size {
        return Err(KernelUpdateError::SizeMismatch);
    }
    let actual_digest: [u8; 32] = Sha256::digest(&archive).into();
    if actual_digest != update.sha256 {
        return Err(KernelUpdateError::DigestMismatch);
    }

    let mut zip = zip::ZipArchive::new(Cursor::new(archive))
        .map_err(|err| KernelUpdateError::Archive(err.to_string()))?;
    let archive_directory = update
        .asset_name
        .strip_suffix(".zip")
        .ok_or_else(|| KernelUpdateError::InvalidRelease("asset is not a ZIP archive".into()))?;
    let staging_directory = create_staging_directory(paths)?;

    for file_name in RUNTIME_FILES {
        let entry_name = format!("{archive_directory}/{file_name}");
        let result = extract_runtime_file(&mut zip, &entry_name, &staging_directory, file_name);
        if let Err(err) = result {
            let _ = std::fs::remove_dir_all(&staging_directory);
            return Err(err);
        }
    }

    let staged_executable = staging_directory.join("sing-box.exe");
    match executable_version(&staged_executable) {
        Ok(version) if version == update.release_version => Ok(PreparedKernelUpdate {
            version,
            staging_directory,
        }),
        Ok(version) => {
            let _ = std::fs::remove_dir_all(&staging_directory);
            Err(KernelUpdateError::InvalidExecutable(format!(
                "expected {}, got {version}",
                update.release_version
            )))
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(&staging_directory);
            Err(KernelUpdateError::InvalidExecutable(err))
        }
    }
}

pub fn install_kernel_update(
    paths: &AppPaths,
    prepared: PreparedKernelUpdate,
) -> Result<Version, KernelUpdateError> {
    let mut backups = Vec::new();
    for file_name in RUNTIME_FILES {
        let target = paths.app_dir().join(file_name);
        if target.exists() {
            let backup = match unused_backup_path(paths, file_name) {
                Ok(backup) => backup,
                Err(message) => {
                    let rollback_errors = restore_runtime_files(&backups, &[]);
                    let _ = std::fs::remove_dir_all(&prepared.staging_directory);
                    return Err(install_error(message, rollback_errors));
                }
            };
            if let Err(err) = std::fs::rename(&target, &backup) {
                let rollback_errors = restore_runtime_files(&backups, &[]);
                let _ = std::fs::remove_dir_all(&prepared.staging_directory);
                return Err(install_error(err.to_string(), rollback_errors));
            }
            backups.push((target, backup));
        }
    }

    let mut installed = Vec::new();
    for file_name in RUNTIME_FILES {
        let staged = prepared.staging_directory.join(file_name);
        let target = paths.app_dir().join(file_name);
        if let Err(err) = std::fs::rename(&staged, &target) {
            let rollback_errors = restore_runtime_files(&backups, &installed);
            let _ = std::fs::remove_dir_all(&prepared.staging_directory);
            return Err(install_error(err.to_string(), rollback_errors));
        }
        installed.push(target);
    }

    for (_, backup) in backups {
        let _ = std::fs::remove_file(backup);
    }
    let _ = std::fs::remove_dir(&prepared.staging_directory);
    Ok(prepared.version.clone())
}

fn parse_release(
    paths: &AppPaths,
    release: GitHubRelease,
    architecture: &str,
) -> Result<KernelUpdate, KernelUpdateError> {
    if release.draft || release.prerelease {
        return Err(KernelUpdateError::InvalidRelease(
            "latest release is not stable".into(),
        ));
    }
    let version_text = release.tag_name.trim_start_matches('v');
    let release_version = Version::parse(version_text)
        .map_err(|err| KernelUpdateError::InvalidRelease(err.to_string()))?;
    let asset_name = format!("sing-box-{version_text}-windows-{architecture}.zip");
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| {
            KernelUpdateError::InvalidRelease(format!("missing release asset {asset_name}"))
        })?;
    if asset.size == 0 || asset.size > MAX_ARCHIVE_SIZE {
        return Err(KernelUpdateError::InvalidRelease(format!(
            "invalid asset size: {} bytes",
            asset.size
        )));
    }
    let expected_url = format!(
        "https://github.com/SagerNet/sing-box/releases/download/{}/{asset_name}",
        release.tag_name
    );
    if asset.browser_download_url != expected_url {
        return Err(KernelUpdateError::InvalidRelease(
            "release asset URL is not an official sing-box download".into(),
        ));
    }
    let digest = asset
        .digest
        .as_deref()
        .and_then(|digest| digest.strip_prefix("sha256:"))
        .ok_or_else(|| KernelUpdateError::InvalidRelease("asset has no SHA-256 digest".into()))?;
    let sha256 = decode_sha256(digest)?;
    let (installed_version, installed_version_error) = if paths.sing_box_exe().exists() {
        match executable_version(&paths.sing_box_exe()) {
            Ok(version) => (Some(version), None),
            Err(err) => (None, Some(err)),
        }
    } else {
        (None, None)
    };

    let runtime_files_complete = RUNTIME_FILES
        .iter()
        .all(|file_name| paths.app_dir().join(file_name).is_file());

    Ok(KernelUpdate {
        installed_version,
        installed_version_error,
        runtime_files_complete,
        release_version,
        asset_name,
        asset_url: asset.browser_download_url,
        asset_size: asset.size,
        sha256,
    })
}

fn release_architecture() -> Result<&'static str, KernelUpdateError> {
    match std::env::consts::ARCH {
        "x86_64" => Ok("amd64"),
        "aarch64" => Ok("arm64"),
        architecture => Err(KernelUpdateError::UnsupportedArchitecture(
            architecture.to_string(),
        )),
    }
}

fn executable_version(path: &Path) -> Result<Version, String> {
    let mut command = Command::new(path);
    command
        .arg("version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_command_window(&mut command);
    let mut child = command.spawn().map_err(|err| err.to_string())?;
    let deadline = Instant::now() + VERSION_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "version command timed out after {} seconds",
                    VERSION_TIMEOUT.as_secs()
                ));
            }
            Err(err) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(err.to_string());
            }
        }
    }
    let output = child.wait_with_output().map_err(|err| err.to_string())?;
    if !output.status.success() {
        return Err(format!("version command exited with {}", output.status));
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_version_output(&text)
        .ok_or_else(|| "version command returned an unrecognized version".into())
}

fn parse_version_output(output: &str) -> Option<Version> {
    output.split_whitespace().find_map(|word| {
        let candidate = word.trim_start_matches('v').trim_matches(|ch: char| {
            !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '+'
        });
        Version::parse(candidate).ok()
    })
}

fn decode_sha256(value: &str) -> Result<[u8; 32], KernelUpdateError> {
    if value.len() != 64 {
        return Err(KernelUpdateError::InvalidRelease(
            "invalid SHA-256 digest length".into(),
        ));
    }
    let mut decoded = [0_u8; 32];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| KernelUpdateError::InvalidRelease("invalid SHA-256 digest".into()))?;
    }
    Ok(decoded)
}

fn create_staging_directory(paths: &AppPaths) -> Result<PathBuf, KernelUpdateError> {
    cleanup_stale_staging_directories(paths);
    for index in 0..100 {
        let path = paths
            .app_dir()
            .join(format!(".sing-box-update-{}-{index}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(KernelUpdateError::Stage(err)),
        }
    }
    Err(KernelUpdateError::Stage(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a staging directory",
    )))
}

fn cleanup_stale_staging_directories(paths: &AppPaths) {
    let Ok(entries) = std::fs::read_dir(paths.app_dir()) else {
        return;
    };
    let current_pid = std::process::id();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(suffix) = name.strip_prefix(".sing-box-update-") else {
            continue;
        };
        let Some((pid, index)) = suffix.split_once('-') else {
            continue;
        };
        let is_stale =
            pid.parse::<u32>().is_ok_and(|pid| pid != current_pid) && index.parse::<u32>().is_ok();
        if is_stale && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

fn extract_runtime_file<R: std::io::Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    entry_name: &str,
    staging_directory: &Path,
    file_name: &str,
) -> Result<(), KernelUpdateError> {
    let mut entry = zip
        .by_name(entry_name)
        .map_err(|_| KernelUpdateError::Archive(format!("missing {entry_name}")))?;
    if !entry.is_file() || entry.size() > MAX_EXTRACTED_FILE_SIZE {
        return Err(KernelUpdateError::Archive(format!(
            "invalid archive entry {entry_name}"
        )));
    }
    let target = staging_directory.join(file_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(KernelUpdateError::Stage)?;
    std::io::copy(&mut entry, &mut file)
        .and_then(|_| file.flush())
        .map_err(KernelUpdateError::Stage)?;
    Ok(())
}

fn unused_backup_path(paths: &AppPaths, file_name: &str) -> Result<PathBuf, String> {
    for index in 0..100 {
        let path = paths.app_dir().join(format!(
            ".{file_name}.singboost-backup-{}-{index}",
            std::process::id()
        ));
        if !path.exists() {
            return Ok(path);
        }
    }
    Err("could not allocate a backup path".into())
}

fn install_error(message: String, rollback_errors: Vec<String>) -> KernelUpdateError {
    let rollback_failed = !rollback_errors.is_empty();
    let message = if rollback_failed {
        format!(
            "{message}; rollback also failed: {}",
            rollback_errors.join("; ")
        )
    } else {
        format!("{message}; previous runtime files were restored")
    };
    KernelUpdateError::Install {
        message,
        rollback_failed,
    }
}

fn restore_runtime_files(backups: &[(PathBuf, PathBuf)], installed: &[PathBuf]) -> Vec<String> {
    let mut errors = Vec::new();
    for path in installed.iter().rev() {
        if let Err(err) = std::fs::remove_file(path) {
            errors.push(format!(
                "failed to remove {}: {err}",
                path.to_string_lossy()
            ));
        }
    }
    for (target, backup) in backups.iter().rev() {
        if let Err(err) = std::fs::rename(backup, target) {
            errors.push(format!(
                "failed to restore {} from {}: {err}",
                target.to_string_lossy(),
                backup.to_string_lossy()
            ));
        }
    }
    errors
}

fn hide_command_window(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = command;
}

#[cfg(test)]
mod tests {
    use super::{
        GitHubRelease, KernelUpdate, PreparedKernelUpdate, decode_sha256, install_kernel_update,
        parse_release, parse_version_output,
    };
    use crate::core::paths::AppPaths;
    use semver::Version;

    #[test]
    fn parses_sing_box_version_output() {
        assert_eq!(
            parse_version_output("sing-box version 1.14.2\nEnvironment: go1.25"),
            Some(Version::new(1, 14, 2))
        );
        assert_eq!(
            parse_version_output("sing-box version 1.15.0-alpha.1"),
            Some(Version::parse("1.15.0-alpha.1").unwrap())
        );
    }

    #[test]
    fn rejects_malformed_sha256() {
        assert!(decode_sha256("abcd").is_err());
        assert!(decode_sha256(&"z".repeat(64)).is_err());
    }

    #[test]
    fn selects_official_asset_for_requested_architecture() {
        let test_root = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(test_root.path().to_path_buf());
        let release: GitHubRelease = serde_json::from_str(
            r#"{
                "tag_name": "v1.14.2",
                "draft": false,
                "prerelease": false,
                "assets": [{
                    "name": "sing-box-1.14.2-windows-amd64.zip",
                    "browser_download_url": "https://github.com/SagerNet/sing-box/releases/download/v1.14.2/sing-box-1.14.2-windows-amd64.zip",
                    "size": 1024,
                    "digest": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                }]
            }"#,
        )
        .unwrap();

        let update = parse_release(&paths, release, "amd64").unwrap();

        assert_eq!(update.installed_version, None);
        assert_eq!(update.installed_version_error, None);
        assert!(!update.runtime_files_complete);
        assert_eq!(update.release_version, Version::new(1, 14, 2));
        assert!(update.update_available());
    }

    #[test]
    fn current_version_is_repairable_when_runtime_files_are_incomplete() {
        let update = KernelUpdate {
            installed_version: Some(Version::new(1, 14, 2)),
            installed_version_error: None,
            runtime_files_complete: false,
            release_version: Version::new(1, 14, 2),
            asset_name: String::new(),
            asset_url: String::new(),
            asset_size: 1,
            sha256: [0; 32],
        };

        assert!(update.update_available());
    }

    #[test]
    fn installs_all_runtime_files_together() {
        let test_root = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(test_root.path().to_path_buf());
        std::fs::write(paths.sing_box_exe(), b"old executable").unwrap();
        std::fs::write(test_root.path().join("libcronet.dll"), b"old dll").unwrap();
        let staging_directory = test_root.path().join("staging");
        std::fs::create_dir(&staging_directory).unwrap();
        std::fs::write(staging_directory.join("sing-box.exe"), b"new executable").unwrap();
        std::fs::write(staging_directory.join("libcronet.dll"), b"new dll").unwrap();
        let prepared = PreparedKernelUpdate {
            version: Version::new(1, 14, 2),
            staging_directory,
        };

        assert_eq!(
            install_kernel_update(&paths, prepared).unwrap(),
            Version::new(1, 14, 2)
        );
        assert_eq!(
            std::fs::read(paths.sing_box_exe()).unwrap(),
            b"new executable"
        );
        assert_eq!(
            std::fs::read(test_root.path().join("libcronet.dll")).unwrap(),
            b"new dll"
        );
    }

    #[test]
    fn failed_replacement_restores_previous_executable() {
        let test_root = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(test_root.path().to_path_buf());
        std::fs::write(paths.sing_box_exe(), b"old executable").unwrap();
        let prepared = PreparedKernelUpdate {
            version: Version::new(1, 14, 2),
            staging_directory: test_root.path().join("missing-staging-directory"),
        };

        let error = install_kernel_update(&paths, prepared).unwrap_err();
        assert!(!error.rollback_failed());
        assert_eq!(
            std::fs::read(paths.sing_box_exe()).unwrap(),
            b"old executable"
        );
    }
}
