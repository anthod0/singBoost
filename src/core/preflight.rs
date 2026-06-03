use crate::core::paths::AppPaths;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreflightError {
    #[error("missing sing-box.exe: {0}")]
    MissingSingBox(PathBuf),
    #[error("missing config.json: {0}")]
    MissingConfig(PathBuf),
    #[error("failed to create logs directory: {0}")]
    LogsDir(io::Error),
    #[error("failed to recreate runtime log: {0}")]
    RuntimeLog(io::Error),
    #[error("failed to read sing-box config: {0}")]
    ConfigRead(io::Error),
    #[error("failed to parse sing-box config JSON: {0}")]
    ConfigParse(serde_json::Error),
}

pub fn validate_preflight_files(paths: &AppPaths) -> Result<(), PreflightError> {
    if !paths.sing_box_exe().exists() {
        return Err(PreflightError::MissingSingBox(paths.sing_box_exe()));
    }
    if !paths.config_json().exists() {
        return Err(PreflightError::MissingConfig(paths.config_json()));
    }
    std::fs::create_dir_all(paths.logs_dir()).map_err(PreflightError::LogsDir)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.runtime_log())
        .map_err(PreflightError::RuntimeLog)?;
    Ok(())
}

pub fn sing_box_tun_enabled(paths: &AppPaths) -> Result<bool, PreflightError> {
    let text = std::fs::read_to_string(paths.config_json()).map_err(PreflightError::ConfigRead)?;
    let config: Value = serde_json::from_str(&text).map_err(PreflightError::ConfigParse)?;
    let Some(inbounds) = config.get("inbounds").and_then(Value::as_array) else {
        return Ok(false);
    };

    Ok(inbounds.iter().any(|inbound| {
        inbound.get("type").and_then(Value::as_str) == Some("tun")
            && inbound.get("enabled").and_then(Value::as_bool) != Some(false)
    }))
}
