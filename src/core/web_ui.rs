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
    #[error(
        "missing an enabled services API dashboard and experimental.clash_api.external_controller"
    )]
    MissingUi,
    #[error("services API dashboard listen address must not be empty")]
    EmptyApiListen,
    #[error("services API dashboard listen_port must not be zero")]
    MissingApiListenPort,
    #[error("experimental.clash_api.external_controller must not be empty")]
    EmptyExternalController,
}

#[derive(Debug, Deserialize)]
struct SingBoxConfig {
    services: Option<Vec<SingBoxService>>,
    experimental: Option<SingBoxExperimental>,
}

#[derive(Debug, Deserialize)]
struct SingBoxService {
    #[serde(rename = "type")]
    service_type: Option<String>,
    listen: Option<String>,
    listen_port: Option<u16>,
    dashboard: Option<ApiDashboard>,
    tls: Option<ApiTls>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiDashboard {
    Enabled(bool),
    Path(String),
    Options(ApiDashboardOptions),
}

impl ApiDashboard {
    fn enabled(&self) -> bool {
        match self {
            Self::Enabled(enabled) => *enabled,
            Self::Path(path) => {
                let _ = path;
                true
            }
            Self::Options(options) => options.enabled,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiDashboardOptions {
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct ApiTls {
    #[serde(default)]
    enabled: bool,
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

    if let Some(service) = config.services.as_deref().and_then(|services| {
        services.iter().find(|service| {
            service.service_type.as_deref() == Some("api")
                && service
                    .dashboard
                    .as_ref()
                    .is_some_and(ApiDashboard::enabled)
        })
    }) {
        return resolve_api_dashboard_url(service);
    }

    let controller = config
        .experimental
        .and_then(|experimental| experimental.clash_api)
        .and_then(|clash_api| clash_api.external_controller)
        .ok_or(WebUiError::MissingUi)?;
    let controller = normalize_external_controller(&controller)?;
    Ok(format!("http://{controller}/ui/"))
}

fn resolve_api_dashboard_url(service: &SingBoxService) -> Result<String, WebUiError> {
    let listen = service
        .listen
        .as_deref()
        .map(str::trim)
        .filter(|listen| !listen.is_empty())
        .ok_or(WebUiError::EmptyApiListen)?;
    let port = service
        .listen_port
        .filter(|port| *port != 0)
        .ok_or(WebUiError::MissingApiListenPort)?;
    let host = normalize_listen_host(listen);
    let scheme = if service.tls.as_ref().is_some_and(|tls| tls.enabled) {
        "https"
    } else {
        "http"
    };

    Ok(format!("{scheme}://{host}:{port}/dashboard/"))
}

fn normalize_listen_host(listen: &str) -> String {
    match listen {
        "0.0.0.0" | "::" | "::0" => "127.0.0.1".to_string(),
        address if address.contains(':') && !address.starts_with('[') => format!("[{address}]"),
        address => address.to_string(),
    }
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
