use crate::core::paths::AppPaths;
use serde::Deserialize;
use std::io;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub start_command: String,
    pub subscription: Option<SubscriptionConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppStateConfig {
    pub run_as_admin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptionConfig {
    pub url: Option<String>,
    pub target: Option<String>,
    pub timeout_secs: Option<u64>,
}

const STATE_FILE_HEADER: &str = "# Managed by SingBoost. Do not edit manually.\n";

impl AppConfig {
    pub fn default_for_app_dir(_app_dir: &Path) -> Self {
        Self {
            start_command: "sing-box.exe -D . -c config.json run".to_string(),
            subscription: None,
        }
    }

    pub fn default_toml() -> &'static str {
        concat!(
            "[sing_box]\n",
            "start_command = 'sing-box.exe -D . -c config.json run'\n\n",
            "# 可选：下载远程完整 sing-box 配置。\n",
            "# 在这里填写地址后，使用托盘菜单：配置 -> 下载远程配置\n",
            "#\n",
            "# [subscription]\n",
            "# url = \"https://example.com/config.json\"\n",
            "# target = \"config.json\"\n",
            "# timeout_secs = 30\n",
        )
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing state run_as_admin")]
    MissingRunAsAdmin,
    #[error("missing sing_box.start_command")]
    MissingStartCommand,
    #[error("subscription.timeout_secs must be between 1 and 300 seconds: {0}")]
    InvalidSubscriptionTimeout(u64),
    #[error("sing_box.start_command must not be empty")]
    EmptyStartCommand,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    sing_box: Option<RawSingBoxConfig>,
    subscription: Option<RawSubscriptionConfig>,
}

#[derive(Debug, Deserialize)]
struct RawStateConfig {
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
    timeout_secs: Option<u64>,
}

pub fn ensure_config_file(paths: &AppPaths) -> io::Result<()> {
    let config_path = paths.config_toml();
    if !config_path.exists() {
        std::fs::write(config_path, AppConfig::default_toml())?;
    }
    Ok(())
}

pub fn ensure_state_file(paths: &AppPaths) -> io::Result<()> {
    let state_path = paths.state_toml();
    if !state_path.exists() {
        std::fs::write(
            state_path,
            format!("{STATE_FILE_HEADER}run_as_admin = false\n"),
        )?;
    }
    Ok(())
}

pub fn load_config(paths: &AppPaths) -> Result<AppConfig, ConfigError> {
    let text = std::fs::read_to_string(paths.config_toml())?;
    let raw: RawConfig = toml::from_str(&text)?;
    let start_command = raw
        .sing_box
        .and_then(|sing_box| sing_box.start_command)
        .ok_or(ConfigError::MissingStartCommand)?;
    if start_command.trim().is_empty() {
        return Err(ConfigError::EmptyStartCommand);
    }
    let subscription = raw
        .subscription
        .map(|subscription| {
            if let Some(timeout_secs) = subscription.timeout_secs {
                if !(1..=300).contains(&timeout_secs) {
                    return Err(ConfigError::InvalidSubscriptionTimeout(timeout_secs));
                }
            }
            Ok(SubscriptionConfig {
                url: subscription.url,
                target: subscription.target,
                timeout_secs: subscription.timeout_secs,
            })
        })
        .transpose()?;
    Ok(AppConfig {
        start_command,
        subscription,
    })
}

pub fn load_state_config(paths: &AppPaths) -> Result<AppStateConfig, ConfigError> {
    let text = std::fs::read_to_string(paths.state_toml())?;
    let raw: RawStateConfig = toml::from_str(&text)?;
    let run_as_admin = raw.run_as_admin.ok_or(ConfigError::MissingRunAsAdmin)?;
    Ok(AppStateConfig { run_as_admin })
}

pub fn save_state_config(paths: &AppPaths, config: &AppStateConfig) -> io::Result<()> {
    std::fs::write(
        paths.state_toml(),
        format!(
            "{STATE_FILE_HEADER}run_as_admin = {}\n",
            config.run_as_admin
        ),
    )
}
