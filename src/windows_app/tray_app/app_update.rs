use super::{TrayApp, UserEvent};
use crate::windows_app::{app_update::start_helper, error_dialog::confirm, show_error, show_info};
use singboost::core::app_update::{
    AppUpdate, PreparedAppUpdate, check_app_update, prepare_app_update,
};
use tao::event_loop::EventLoopProxy;

impl TrayApp {
    pub(super) fn check_for_app_update(&mut self, proxy: EventLoopProxy<UserEvent>) {
        if self.app_updating || self.kernel_updating || self.subscription_downloading {
            return;
        }
        self.app_updating = true;
        self.update_menu();
        self.log("checking for SingBoost updates");
        std::thread::spawn(move || {
            let result = check_app_update().map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::AppUpdateChecked(result));
        });
    }

    pub(super) fn finish_app_update_check(
        &mut self,
        result: Result<AppUpdate, String>,
        proxy: EventLoopProxy<UserEvent>,
    ) {
        let update = match result {
            Ok(update) => update,
            Err(e) => {
                self.app_update_error(&e);
                return;
            }
        };
        if !update.available() {
            self.app_updating = false;
            self.update_menu();
            show_info("SingBoost 更新", "当前已是最新版本，或高于最新稳定版。");
            return;
        }
        if !confirm(
            "SingBoost 更新",
            &format!(
                "当前版本：{}\n最新版本：{}\n\n是否下载并更新？更新会停止 sing-box 并重启 SingBoost，不修改配置文件。",
                env!("CARGO_PKG_VERSION"),
                update.version
            ),
        ) {
            self.app_updating = false;
            self.update_menu();
            return;
        }
        let app_dir = self.paths.app_dir();
        std::thread::spawn(move || {
            let result = (|| {
                let mut prepared =
                    prepare_app_update(&app_dir, &update).map_err(|e| e.to_string())?;
                start_helper(&mut prepared).map_err(|e| e.to_string())?;
                Ok(prepared)
            })();
            let _ = proxy.send_event(UserEvent::AppUpdatePrepared(result));
        });
    }

    pub(super) fn finish_app_update_prepare(&mut self, result: Result<PreparedAppUpdate, String>) {
        match result {
            Ok(_prepared) => {
                self.log("SingBoost update helper ready; exiting for replacement");
                self.exit();
            }
            Err(e) => self.app_update_error(&e),
        }
    }

    fn app_update_error(&mut self, error: &str) {
        self.app_updating = false;
        self.update_menu();
        let message = format!("SingBoost 更新失败：{error}");
        self.log(&message);
        show_error(&message);
    }
}
