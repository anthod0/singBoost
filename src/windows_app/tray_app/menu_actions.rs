use super::TrayApp;
use super::config::write_config;
use crate::windows_app::autostart::{autostart_enabled, remove_autostart, set_autostart};
use crate::windows_app::elevation::{is_elevated, relaunch_elevated};
use crate::windows_app::error_dialog::{confirm, show_error};
use crate::windows_app::process::terminate_child;
use crate::windows_app::show_info;
use crate::windows_app::subscription_dialog::show_subscription_dialog;
use crate::windows_app::tray_menu::{
    ABOUT_ID, ADMIN_ID, AUTOSTART_ID, EXIT_ID, LOG_ID, OPEN_UI_ID, REMOTE_CONFIG_ID, RESTART_ID,
    START_STOP_ID,
};
use singboost::{
    AppState, SubscriptionConfig, download_subscription, load_config, resolve_subscription_target,
    resolve_web_ui_url, save_subscription_url,
};
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
            OPEN_UI_ID => self.open_web_ui(),
            LOG_ID => self.open_log_window(),
            REMOTE_CONFIG_ID => self.configure_remote_config(),
            ADMIN_ID => self.toggle_admin(),
            AUTOSTART_ID => self.toggle_autostart(),
            ABOUT_ID => self.show_about(),
            EXIT_ID => self.exit(),
            _ => {}
        }
        self.update_menu();
    }

    pub(super) fn open_web_ui(&mut self) {
        match resolve_web_ui_url(&self.paths) {
            Ok(url) => {
                if let Err(err) = open::that(&url) {
                    self.error(&format!("打开 UI 失败：{err}"));
                }
            }
            Err(err) => self.error(&format!("打开 UI 失败：无法解析 UI 地址：{err}")),
        }
    }

    fn configure_remote_config(&mut self) {
        let initial_url = self
            .config
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.url.as_deref())
            .unwrap_or("");
        let Some(url) = show_subscription_dialog(initial_url) else {
            return;
        };
        let url = url.trim();
        if url.is_empty() {
            show_error("订阅地址不能为空。");
            return;
        }
        let target = self
            .config
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.target.clone());
        let target_path = match resolve_subscription_target(&self.paths, target.as_deref()) {
            Ok(target_path) => target_path,
            Err(err) => {
                let message = format!("远程配置目标无效：{err}");
                self.log(&message);
                show_error(&message);
                return;
            }
        };
        if target_path.exists()
            && !confirm(
                "确认覆盖",
                &format!(
                    "目标文件 {} 已存在，是否覆盖？",
                    target_path.to_string_lossy()
                ),
            )
        {
            return;
        }
        if let Err(err) = save_subscription_url(&self.paths, url) {
            let message = format!("保存远程配置失败：{err}");
            self.log(&message);
            show_error(&message);
            return;
        }
        self.config = match load_config(&self.paths) {
            Ok(config) => config,
            Err(err) => {
                let message = format!("重新读取配置失败：{err}");
                self.log(&message);
                show_error(&message);
                return;
            }
        };
        let subscription = self
            .config
            .subscription
            .clone()
            .unwrap_or(SubscriptionConfig {
                url: Some(url.to_string()),
                target,
            });
        match download_subscription(&self.paths, &subscription) {
            Ok(target) => self.log(&format!("远程配置已下载：{}", target.to_string_lossy())),
            Err(err) => {
                let message = format!("下载远程配置失败：{err}");
                self.log(&message);
                show_error(&message);
            }
        }
    }

    fn show_about(&self) {
        show_info(
            "About SingBoost",
            &format!(
                "SingBoost {}\n\nA minimal Windows tray launcher for sing-box.\n\nLicense: MIT",
                env!("CARGO_PKG_VERSION")
            ),
        );
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
