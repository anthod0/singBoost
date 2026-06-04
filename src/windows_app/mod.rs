#![cfg(windows)]

mod autostart;
mod elevation;
mod error_dialog;
mod process;
mod single_instance;
mod tray_app;
mod tray_menu;

pub(crate) use error_dialog::{show_error, show_info};

use singboost::{AppPaths, ensure_config_file, ensure_state_file, load_config, load_state_config};
use std::error::Error;
use tray_app::TrayApp;

pub fn run() -> Result<(), Box<dyn Error>> {
    let paths = AppPaths::from_current_exe()?;
    ensure_config_file(&paths)?;
    ensure_state_file(&paths)?;
    let config = load_config(&paths)?;
    let state_config = load_state_config(&paths)?;

    if state_config.run_as_admin && !elevation::is_elevated() {
        elevation::relaunch_elevated()?;
        return Ok(());
    }

    let _single_instance = single_instance::acquire()?;
    let _ = autostart::repair_autostart_if_stale(&state_config);
    let app = TrayApp::new(paths, config, state_config)?;
    app.run();
}
