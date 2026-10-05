use crate::core::config::SubscriptionConfig;
use crate::core::http::get_bytes_with_basic_auth;
use crate::core::paths::{AppPaths, append_child, looks_like_windows_path};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_SUBSCRIPTION_DOWNLOAD_TIMEOUT_SECS: u64 = 30;
const MAX_SUBSCRIPTION_DOWNLOAD_SIZE: u64 = 100 * 1024 * 1024;

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
    let timeout = subscription_download_timeout(subscription);
    subscription
        .validate_basic_auth()
        .map_err(|err| SubscriptionError::Download(err.to_string()))?;
    let credentials = subscription
        .username
        .as_deref()
        .zip(subscription.password.as_deref());
    let body = get_bytes_with_basic_auth(url, timeout, MAX_SUBSCRIPTION_DOWNLOAD_SIZE, credentials)
        .map_err(|err| SubscriptionError::Download(err.to_string()))?;
    write_subscription_content(&target, &body)?;
    Ok(target)
}

fn subscription_download_timeout(subscription: &SubscriptionConfig) -> Duration {
    Duration::from_secs(
        subscription
            .timeout_secs
            .unwrap_or(DEFAULT_SUBSCRIPTION_DOWNLOAD_TIMEOUT_SECS),
    )
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
