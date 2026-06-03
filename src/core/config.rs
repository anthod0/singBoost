use crate::core::paths::AppPaths;
use serde::Deserialize;
use std::io;
use std::path::Path;
use thiserror::Error;

const DEFAULT_SUBSCRIPTION_TARGET: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub run_as_admin: bool,
    pub start_command: String,
    pub subscription: Option<SubscriptionConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionConfig {
    pub url: Option<String>,
    pub target: Option<String>,
}

impl AppConfig {
    pub fn default_for_app_dir(_app_dir: &Path) -> Self {
        Self {
            run_as_admin: false,
            start_command: "sing-box.exe -D . -c config.json run".to_string(),
            subscription: None,
        }
    }

    pub fn default_toml() -> &'static str {
        concat!(
            "[app]\n",
            "run_as_admin = false\n\n",
            "[sing_box]\n",
            "start_command = 'sing-box.exe -D . -c config.json run'\n",
        )
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing app.run_as_admin")]
    MissingRunAsAdmin,
    #[error("missing sing_box.start_command")]
    MissingStartCommand,
    #[error("sing_box.start_command must not be empty")]
    EmptyStartCommand,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    app: Option<RawAppConfig>,
    sing_box: Option<RawSingBoxConfig>,
    subscription: Option<RawSubscriptionConfig>,
}

#[derive(Debug, Deserialize)]
struct RawAppConfig {
    run_as_admin: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawSingBoxConfig {
    start_command: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSubscriptionConfig {
    url: Option<String>,
    target: Option<String>,
}

pub fn ensure_config_file(paths: &AppPaths) -> io::Result<()> {
    let config_path = paths.config_toml();
    if !config_path.exists() {
        std::fs::write(config_path, AppConfig::default_toml())?;
    }
    Ok(())
}

pub fn load_config(paths: &AppPaths) -> Result<AppConfig, ConfigError> {
    let text = std::fs::read_to_string(paths.config_toml())?;
    let raw: RawConfig = toml::from_str(&text)?;
    let run_as_admin = raw
        .app
        .and_then(|app| app.run_as_admin)
        .ok_or(ConfigError::MissingRunAsAdmin)?;
    let start_command = raw
        .sing_box
        .and_then(|sing_box| sing_box.start_command)
        .ok_or(ConfigError::MissingStartCommand)?;
    if start_command.trim().is_empty() {
        return Err(ConfigError::EmptyStartCommand);
    }
    let subscription = raw.subscription.map(|subscription| SubscriptionConfig {
        url: subscription.url,
        target: subscription.target,
    });
    Ok(AppConfig {
        run_as_admin,
        start_command,
        subscription,
    })
}

pub fn save_subscription_url(paths: &AppPaths, url: &str) -> io::Result<()> {
    let text = std::fs::read_to_string(paths.config_toml())?;
    let mut value: toml::Value =
        toml::from_str(&text).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let root = value.as_table_mut().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "boost.toml root is not a table")
    })?;
    let subscription = root
        .entry("subscription".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let subscription = subscription.as_table_mut().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "boost.toml subscription section is not a table",
        )
    })?;
    subscription.insert("url".to_string(), toml::Value::String(url.to_string()));
    subscription
        .entry("target".to_string())
        .or_insert_with(|| toml::Value::String(DEFAULT_SUBSCRIPTION_TARGET.to_string()));
    std::fs::write(
        paths.config_toml(),
        toml::to_string_pretty(&value).map_err(io::Error::other)?,
    )
}
