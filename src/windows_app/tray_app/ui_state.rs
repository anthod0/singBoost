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
