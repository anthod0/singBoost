use super::{TrayApp, UserEvent};
use crate::windows_app::error_dialog::{confirm, show_error};
use crate::windows_app::show_info;
use singboost::{
    AppState, KernelUpdate, PreparedKernelUpdate, check_kernel_update, install_kernel_update,
    prepare_kernel_update,
};
use tao::event_loop::EventLoopProxy;

impl TrayApp {
    pub(super) fn check_for_kernel_update(&mut self, event_proxy: EventLoopProxy<UserEvent>) {
        if self.kernel_updating {
            return;
        }
        self.kernel_updating = true;
        self.update_menu();
        self.log("checking for sing-box updates");

        let paths = self.paths.clone();
        std::thread::spawn(move || {
            let result = check_kernel_update(&paths).map_err(|err| err.to_string());
            let _ = event_proxy.send_event(UserEvent::KernelUpdateChecked(result));
        });
    }

    pub(super) fn finish_kernel_update_check(
        &mut self,
        result: Result<KernelUpdate, String>,
        event_proxy: EventLoopProxy<UserEvent>,
    ) {
        let update = match result {
            Ok(update) => update,
            Err(err) => {
                self.finish_kernel_update_error(&format!("检查内核更新失败：{err}"));
                return;
            }
        };
        match (&update.installed_version, &update.installed_version_error) {
            (Some(version), _) => self
                .menu
                .kernel_version
                .set_text(format!("当前版本：{version}")),
            (None, Some(_)) => self.menu.kernel_version.set_text("当前版本：无法识别"),
            (None, None) => self.menu.kernel_version.set_text("当前版本：未安装"),
        }

        if !update.update_available() {
            self.kernel_updating = false;
            self.update_menu();
            let message = match &update.installed_version {
                Some(installed) if installed > &update.release_version => format!(
                    "当前 sing-box 版本 {installed} 高于最新稳定版 {}。",
                    update.release_version
                ),
                _ => format!("当前已是最新稳定版 {}。", update.release_version),
            };
            self.log(&message);
            show_info("内核更新", &message);
            return;
        }

        let message = match (&update.installed_version, &update.installed_version_error) {
            (Some(installed), _) if installed == &update.release_version => format!(
                "当前 sing-box 版本为 {installed}，但运行文件不完整。\n\n是否重新下载并修复？"
            ),
            (Some(installed), _) => format!(
                "发现 sing-box 新版本。\n\n当前版本：{installed}\n最新版本：{}\n\n是否下载并更新？",
                update.release_version
            ),
            (None, Some(error)) => format!(
                "现有 sing-box 版本无法识别：{error}\n\n最新稳定版：{}\n\n是否下载并替换？",
                update.release_version
            ),
            (None, None) => format!(
                "尚未安装 sing-box。\n\n最新稳定版：{}\n\n是否下载并安装？",
                update.release_version
            ),
        };
        if !confirm("内核更新", &message) {
            self.kernel_updating = false;
            self.update_menu();
            return;
        }

        self.update_menu();
        let paths = self.paths.clone();
        std::thread::spawn(move || {
            let result = prepare_kernel_update(&paths, &update).map_err(|err| err.to_string());
            let _ = event_proxy.send_event(UserEvent::KernelUpdatePrepared(result));
        });
    }

    pub(super) fn finish_kernel_update_prepare(
        &mut self,
        result: Result<PreparedKernelUpdate, String>,
    ) {
        let prepared = match result {
            Ok(prepared) => prepared,
            Err(err) => {
                self.finish_kernel_update_error(&format!("下载内核失败：{err}"));
                return;
            }
        };

        let restart = self.state == AppState::Running;
        if restart {
            self.stop_kernel();
            self.log("sing-box stopped for kernel update");
        }
        let paths = self.paths.clone();
        let result = install_kernel_update(&paths, prepared);
        self.kernel_updating = false;

        match result {
            Ok(version) => {
                self.menu
                    .kernel_version
                    .set_text(format!("当前版本：{version}"));
                let message = format!("sing-box {version} 已安装。");
                self.log(&message);
                if restart {
                    self.start_kernel();
                } else {
                    self.update_menu();
                }
                show_info("内核更新", &message);
            }
            Err(err) => {
                let rollback_failed = err.rollback_failed();
                let message = format!("安装内核失败：{err}");
                self.log(&message);
                if restart && !rollback_failed {
                    self.start_kernel();
                } else {
                    self.update_menu();
                }
                show_error(&message);
            }
        }
    }

    fn finish_kernel_update_error(&mut self, message: &str) {
        self.kernel_updating = false;
        self.update_menu();
        self.log(message);
        show_error(message);
    }
}
