use crate::core::config::SubscriptionConfig;
use crate::core::paths::{AppPaths, append_child, looks_like_windows_path};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Error)]
pub enum SubscriptionError {
    #[error("subscription URL is empty")]
    EmptyUrl,
    #[error("subscription target is missing")]
    MissingTarget,
    #[error("subscription target is empty")]
    EmptyTarget,
    #[error("subscription target must be a relative path inside the application directory: {0}")]
    InvalidTarget(String),
    #[error("remote config response is empty")]
    EmptyResponse,
    #[error("remote config is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("download failed: {0}")]
    Download(String),
    #[error("failed to write remote config: {0}")]
    Write(#[from] std::io::Error),
}

pub fn resolve_subscription_target(
    paths: &AppPaths,
    target: Option<&str>,
) -> Result<PathBuf, SubscriptionError> {
    let target = target.ok_or(SubscriptionError::MissingTarget)?.trim();
    if target.is_empty() {
        return Err(SubscriptionError::EmptyTarget);
    }
    let path = Path::new(target);
    if path.is_absolute() || looks_like_windows_path(path) {
        return Err(SubscriptionError::InvalidTarget(target.to_string()));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(SubscriptionError::InvalidTarget(target.to_string()));
    }
    Ok(append_child(&paths.app_dir(), target))
}

pub fn download_subscription(
    paths: &AppPaths,
    subscription: &SubscriptionConfig,
) -> Result<PathBuf, SubscriptionError> {
    let url = subscription
        .url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .ok_or(SubscriptionError::EmptyUrl)?;
    let target = resolve_subscription_target(paths, subscription.target.as_deref())?;
    let body = download_body(url)?;
    write_subscription_content(&target, &body)?;
    Ok(target)
}

#[cfg(not(windows))]
fn download_body(url: &str) -> Result<Vec<u8>, SubscriptionError> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|err| SubscriptionError::Download(err.to_string()))?;
    response
        .body_mut()
        .read_to_vec()
        .map_err(|err| SubscriptionError::Download(err.to_string()))
}

#[cfg(windows)]
fn download_body(url: &str) -> Result<Vec<u8>, SubscriptionError> {
    use std::os::windows::process::CommandExt;

    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "$url=$env:SINGBOOST_SUBSCRIPTION_URL; $bytes=(New-Object System.Net.WebClient).DownloadData($url); [Console]::OpenStandardOutput().Write($bytes,0,$bytes.Length)",
        ])
        .env("SINGBOOST_SUBSCRIPTION_URL", url)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| SubscriptionError::Download(err.to_string()))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(SubscriptionError::Download(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

pub fn write_subscription_content(target: &Path, content: &[u8]) -> Result<(), SubscriptionError> {
    if content.iter().all(|byte| byte.is_ascii_whitespace()) {
        return Err(SubscriptionError::EmptyResponse);
    }
    let config: serde_json::Value = serde_json::from_slice(content)?;
    let pretty_config = serde_json::to_string_pretty(&config)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension(format!(
        "{}.tmp",
        target
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("download")
    ));
    std::fs::write(&tmp, pretty_config)?;
    if target.exists() {
        std::fs::remove_file(target)?;
    }
    std::fs::rename(tmp, target)?;
    Ok(())
}
