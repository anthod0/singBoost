pub mod command;
pub mod config;
pub mod paths;
pub mod preflight;
pub mod runtime_log;
pub mod subscription;
pub mod web_ui;

pub use command::{CommandLineError, KernelCommand, spawn_command_line};
pub use config::{
    AppConfig, ConfigError, SubscriptionConfig, ensure_config_file, load_config,
    save_subscription_url,
};
pub use paths::AppPaths;
pub use preflight::{PreflightError, sing_box_tun_enabled, validate_preflight_files};
pub use runtime_log::RuntimeLog;
pub use subscription::{
    SubscriptionError, download_subscription, resolve_subscription_target,
    write_subscription_content,
};
pub use web_ui::{WebUiError, resolve_web_ui_url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Stopped,
    Starting,
    Running,
    Error,
}
