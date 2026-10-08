use super::TrayApp;
use crate::windows_app::autostart::autostart_enabled;
use crate::windows_app::error_dialog::show_error;
use singboost::AppState;

impl TrayApp {
    pub(super) fn update_menu(&self) {
        match self.state {
            AppState::Running => {
                self.menu.start_stop.set_text("停止");
                self.menu.start_stop.set_enabled(true);
                self.menu.restart.set_enabled(true);
                self.menu.open_ui.set_enabled(true);
            }
            AppState::Starting => {
                self.menu.start_stop.set_text("启动中...");
                self.menu.start_stop.set_enabled(false);
                self.menu.restart.set_enabled(false);
                self.menu.open_ui.set_enabled(false);
            }
            AppState::Stopped | AppState::Error => {
                self.menu.start_stop.set_text("启动");
                self.menu.start_stop.set_enabled(true);
                self.menu.restart.set_enabled(false);
                self.menu.open_ui.set_enabled(false);
            }
        }
        if self.subscription_downloading {
            self.menu.download_remote_config.set_text("下载中...");
            self.menu.download_remote_config.set_enabled(false);
        } else {
            self.menu.download_remote_config.set_text("下载远程配置");
            self.menu.download_remote_config.set_enabled(true);
        }
        if self.kernel_updating {
            self.menu.download_remote_config.set_enabled(false);
            self.menu.update_kernel.set_text("处理中...");
            self.menu.update_kernel.set_enabled(false);
            self.menu.start_stop.set_enabled(false);
            self.menu.restart.set_enabled(false);
        } else {
            self.menu.update_kernel.set_text("检查内核更新");
            self.menu.update_kernel.set_enabled(true);
        }
        self.menu.update_app.set_enabled(
            !self.app_updating && !self.kernel_updating && !self.subscription_downloading,
        );
        self.menu.update_app.set_text(if self.app_updating {
            "SingBoost 更新中..."
        } else {
            "检查 SingBoost 更新"
        });
        if self.app_updating {
            self.menu.update_kernel.set_enabled(false);
            self.menu.download_remote_config.set_enabled(false);
            self.menu.admin.set_enabled(false);
            self.menu.autostart.set_enabled(false);
        } else {
            self.menu.admin.set_enabled(true);
            self.menu.autostart.set_enabled(true);
        }
        self.menu.admin.set_checked(self.state_config.run_as_admin);
        self.menu.autostart.set_checked(autostart_enabled());
    }

    pub(super) fn log(&self, message: &str) {
        if let Ok(mut log) = self.runtime_log.lock() {
            let _ = log.append_event(message);
        }
    }

    pub(super) fn error(&mut self, message: &str) {
        self.state = AppState::Error;
        self.update_menu();
        self.log(message);
        show_error(message);
    }
}
