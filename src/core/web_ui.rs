use crate::core::paths::AppPaths;
use serde::Deserialize;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WebUiError {
    #[error("failed to read sing-box config: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse sing-box config JSON: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("missing experimental.clash_api.external_controller")]
    MissingExternalController,
    #[error("experimental.clash_api.external_controller must not be empty")]
    EmptyExternalController,
}

#[derive(Debug, Deserialize)]
struct SingBoxConfig {
    experimental: Option<SingBoxExperimental>,
}

#[derive(Debug, Deserialize)]
struct SingBoxExperimental {
    clash_api: Option<SingBoxClashApi>,
}

#[derive(Debug, Deserialize)]
struct SingBoxClashApi {
    external_controller: Option<String>,
}

pub fn resolve_web_ui_url(paths: &AppPaths) -> Result<String, WebUiError> {
    let text = std::fs::read_to_string(paths.config_json())?;
    let config: SingBoxConfig = serde_json::from_str(&text)?;
    let controller = config
        .experimental
        .and_then(|experimental| experimental.clash_api)
        .and_then(|clash_api| clash_api.external_controller)
        .ok_or(WebUiError::MissingExternalController)?;
    let controller = normalize_external_controller(&controller)?;
    Ok(format!("http://{controller}/ui/"))
}

fn normalize_external_controller(controller: &str) -> Result<String, WebUiError> {
    let controller = controller.trim();
    if controller.is_empty() {
        return Err(WebUiError::EmptyExternalController);
    }

    let without_scheme = controller
        .strip_prefix("http://")
        .or_else(|| controller.strip_prefix("https://"))
        .unwrap_or(controller);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);

    if let Some(port) = authority.strip_prefix(':') {
        return Ok(format!("127.0.0.1:{port}"));
    }

    let normalized = if let Some(port) = authority.strip_prefix("0.0.0.0:") {
        format!("127.0.0.1:{port}")
    } else if let Some(port) = authority.strip_prefix("[::]:") {
        format!("127.0.0.1:{port}")
    } else if let Some(port) = authority.strip_prefix("[::0]:") {
        format!("127.0.0.1:{port}")
    } else {
        authority.to_string()
    };

    Ok(normalized)
}
