use super::TrayApp;
use super::config::write_config;
use crate::windows_app::autostart::{autostart_enabled, remove_autostart, set_autostart};
use crate::windows_app::elevation::{is_elevated, relaunch_elevated};
use crate::windows_app::process::terminate_child;
use crate::windows_app::tray_menu::{
    ADMIN_ID, AUTOSTART_ID, EXIT_ID, LOG_ID, OPEN_UI_ID, RESTART_ID, START_STOP_ID,
};
use singboost::{AppState, resolve_web_ui_url};
use std::error::Error;
use std::process::Command;
use tray_icon::menu::MenuId;

impl TrayApp {
    pub(super) fn handle_menu(&mut self, id: MenuId) {
        match id.as_ref() {
            START_STOP_ID => match self.state {
                AppState::Running => self.stop_kernel(),
                AppState::Stopped | AppState::Error => self.start_kernel(),
                AppState::Starting => {}
            },
            RESTART_ID => self.restart_kernel(),
            OPEN_UI_ID => match resolve_web_ui_url(&self.paths) {
                Ok(url) => {
                    if let Err(err) = open::that(&url) {
                        self.error(&format!("打开 UI 失败：{err}"));
                    }
                }
                Err(err) => self.error(&format!("打开 UI 失败：无法解析 UI 地址：{err}")),
            },
            LOG_ID => self.open_log_window(),
            ADMIN_ID => self.toggle_admin(),
            AUTOSTART_ID => self.toggle_autostart(),
            EXIT_ID => self.exit(),
            _ => {}
        }
        self.update_menu();
    }

    fn open_log_window(&mut self) {
        let script = format!(
            "Get-Content -LiteralPath '{}' -Wait",
            self.paths
                .runtime_log()
                .to_string_lossy()
                .replace('\'', "''")
        );
        match Command::new("powershell.exe")
            .args(["-NoExit", "-Command", &script])
            .current_dir(self.paths.app_dir())
            .spawn()
        {
            Ok(child) => self.log_windows.push(child),
            Err(err) => self.error(&format!("打开日志窗口失败：{err}")),
        }
    }

    fn toggle_admin(&mut self) {
        let enabled = !self.config.run_as_admin;
        self.config.run_as_admin = enabled;
        if let Err(err) = write_config(&self.paths, &self.config) {
            self.error(&format!("写入配置失败：{err}"));
            return;
        }
        if autostart_enabled() {
            if let Err(err) = set_autostart(&self.paths, self.config.run_as_admin) {
                self.error(&format!(
                    "管理员运行配置已更新，但同步开机自启任务失败：{err}。请重新切换“开机自启”，或检查 Windows 任务计划。"
                ));
            }
        }
        if enabled && !is_elevated() {
            let paths = self.paths.clone();
            self.exit_after(|| relaunch_elevated(&paths));
        }
    }

    fn toggle_autostart(&mut self) {
        if autostart_enabled() {
            if let Err(err) = remove_autostart() {
                self.error(&format!("关闭开机自启失败：{err}"));
            }
        } else if let Err(err) = set_autostart(&self.paths, self.config.run_as_admin) {
            self.error(&format!("启用开机自启失败：{err}"));
        }
    }

    fn exit(&mut self) -> ! {
        self.stop_kernel();
        for child in &mut self.log_windows {
            terminate_child(child);
        }
        std::process::exit(0)
    }

    fn exit_after<F>(&mut self, f: F) -> !
    where
        F: FnOnce() -> Result<(), Box<dyn Error>>,
    {
        let result = f();
        if let Err(err) = result {
            self.error(&format!("退出前操作失败：{err}"));
        }
        self.exit()
    }
}
