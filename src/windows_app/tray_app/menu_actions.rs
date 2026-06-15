use super::TrayApp;
use crate::windows_app::autostart::{autostart_enabled, remove_autostart, set_autostart};
use crate::windows_app::elevation::{is_elevated, relaunch_elevated};
use crate::windows_app::error_dialog::{confirm, show_error};
use crate::windows_app::process::terminate_child;
use crate::windows_app::show_info;
use crate::windows_app::tray_menu::{
    ABOUT_ID, ADMIN_ID, AUTOSTART_ID, DOWNLOAD_REMOTE_CONFIG_ID, EXIT_ID, LOG_ID, OPEN_APP_DIR_ID,
    OPEN_CONFIG_ID, OPEN_SING_BOX_CONFIG_ID, OPEN_UI_ID, RESTART_ID, START_STOP_ID,
};
use singboost::{
    AppState, download_subscription, load_config, resolve_subscription_target, resolve_web_ui_url,
    save_state_config,
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
            OPEN_CONFIG_ID => self.open_config_file(),
            OPEN_APP_DIR_ID => self.open_app_dir(),
            OPEN_SING_BOX_CONFIG_ID => self.open_sing_box_config_file(),
            DOWNLOAD_REMOTE_CONFIG_ID => self.download_remote_config(),
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

    fn open_config_file(&mut self) {
        self.open_path(self.paths.config_toml(), "打开配置文件失败");
    }

    fn open_app_dir(&mut self) {
        self.open_path(self.paths.app_dir(), "打开程序目录失败");
    }

    fn open_sing_box_config_file(&mut self) {
        let path = self
            .config
            .subscription
            .as_ref()
            .and_then(|subscription| subscription.target.as_deref())
            .map(|target| resolve_subscription_target(&self.paths, Some(target)))
            .transpose()
            .map(|target| target.unwrap_or_else(|| self.paths.config_json()));

        match path {
            Ok(path) => self.open_path(path, "打开 sing-box 配置文件失败"),
            Err(err) => self.error(&format!("打开 sing-box 配置文件失败：{err}")),
        }
    }

    fn open_path(&mut self, path: std::path::PathBuf, context: &str) {
        if let Err(err) = open::that(&path) {
            self.error(&format!("{context}：{err}"));
        }
    }

    fn download_remote_config(&mut self) {
        self.config = match load_config(&self.paths) {
            Ok(config) => config,
            Err(err) => {
                let message = format!("重新读取配置失败：{err}");
                self.log(&message);
                show_error(&message);
                return;
            }
        };
        let Some(subscription) = self.config.subscription.clone() else {
            let message =
                "下载远程配置失败：缺少 subscription 配置，请先编辑 boost.toml".to_string();
            self.log(&message);
            show_error(&message);
            return;
        };
        let target_path =
            match resolve_subscription_target(&self.paths, subscription.target.as_deref()) {
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
        match download_subscription(&self.paths, &subscription) {
            Ok(target) => {
                let message = format!("远程配置已下载：{}", target.to_string_lossy());
                self.log(&message);
                if confirm("下载成功", &format!("{message}\n\n是否重启 sing-box？")) {
                    self.restart_kernel();
                }
            }
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
            "Get-Content -LiteralPath '{}' -Encoding UTF8 -Tail 100 -Wait",
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
        let enabled = !self.state_config.run_as_admin;
        self.state_config.run_as_admin = enabled;
        if let Err(err) = save_state_config(&self.paths, &self.state_config) {
            self.error(&format!("写入状态失败：{err}"));
            return;
        }
        if autostart_enabled() {
            if let Err(err) = set_autostart(&self.paths, self.state_config.run_as_admin) {
                self.error(&format!(
                    "管理员运行配置已更新，但同步开机自启任务失败：{err}。请重新切换“开机自启”，或检查 Windows 任务计划。"
                ));
            }
        }
        if enabled && !is_elevated() {
            self.exit_after(|| relaunch_elevated());
        }
    }

    fn toggle_autostart(&mut self) {
        if autostart_enabled() {
            if let Err(err) = remove_autostart() {
                self.error(&format!("关闭开机自启失败：{err}"));
            }
        } else if let Err(err) = set_autostart(&self.paths, self.state_config.run_as_admin) {
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
