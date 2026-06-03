use crate::core::paths::AppPaths;
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
