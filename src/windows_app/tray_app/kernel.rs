use super::TrayApp;
use crate::windows_app::elevation::is_elevated;
use crate::windows_app::error_dialog::show_info;
use crate::windows_app::process::{hide_window, pipe_reader, terminate_child};
use singboost::{
    AppState, KernelCommand, PreflightError, sing_box_tun_enabled, validate_preflight_files,
};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;

impl TrayApp {
    pub(super) fn start_kernel(&mut self) {
        if matches!(self.state, AppState::Running | AppState::Starting) {
            return;
        }
        self.state = AppState::Starting;
        self.update_menu();

        if let Err(err) = validate_preflight_files(&self.paths) {
            match err {
                PreflightError::MissingSingBox(_) => {
                    self.kernel_start_info("未找到 sing-box 内核");
                }
                PreflightError::MissingConfig(path) => {
                    self.kernel_start_info(&format!(
                        "未找到 sing-box 配置文件, {}",
                        path.to_string_lossy()
                    ));
                }
                err => self.kernel_start_error(&format!("启动前检查失败：{err}")),
            }
            return;
        }
        let command = match KernelCommand::run(&self.paths, &self.config) {
            Ok(command) => command,
            Err(err) => {
                self.kernel_start_error(&format!(
                    "启动命令无效：{err}。请检查 boost.toml 中的 sing_box.start_command。"
                ));
                return;
            }
        };
        if let Err(err) = self.run_check() {
            self.kernel_start_error(&format!("sing-box check 失败：{err}"));
            return;
        }
        match sing_box_tun_enabled(&self.paths) {
            Ok(true) if !is_elevated() => {
                self.kernel_start_error("sing-box 配置启用了 TUN 模式，需要管理员权限。请启用“以管理员身份运行”，或以管理员身份启动 SingBoost。");
                return;
            }
            Ok(_) => {}
            Err(err) => {
                self.kernel_start_error(&format!("检查 TUN 模式失败：{err}"));
                return;
            }
        }

        let mut child_command = Command::new(&command.program);
        hide_window(
            child_command
                .args(&command.args)
                .current_dir(self.paths.app_dir())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        );
        match child_command.spawn() {
            Ok(mut child) => {
                self.pipe_child_logs(&mut child);
                self.log("sing-box started");
                self.kernel = Some(child);
                self.state = AppState::Running;
                self.update_menu();
            }
            Err(err) => self.kernel_start_error(&format!("启动 sing-box 进程失败：{err}")),
        }
    }

    fn run_check(&mut self) -> Result<(), String> {
        let command = KernelCommand::check(&self.paths);
        let mut check_command = Command::new(&command.program);
        hide_window(
            check_command
                .args(&command.args)
                .current_dir(self.paths.app_dir()),
        );
        let output = check_command.output().map_err(|err| err.to_string())?;
        self.log(&String::from_utf8_lossy(&output.stdout));
        self.log(&String::from_utf8_lossy(&output.stderr));
        if output.status.success() {
            Ok(())
        } else {
            Err(format!("exit status {}", output.status))
        }
    }

    pub(super) fn stop_kernel(&mut self) {
        if let Some(mut child) = self.kernel.take() {
            terminate_child(&mut child);
            self.log("sing-box stopped");
        }
        self.state = AppState::Stopped;
    }

    pub(super) fn restart_kernel(&mut self) {
        self.stop_kernel();
        self.start_kernel();
    }

    fn pipe_child_logs(&self, child: &mut Child) {
        if let Some(stdout) = child.stdout.take() {
            pipe_reader(stdout, Arc::clone(&self.runtime_log), "stdout");
        }
        if let Some(stderr) = child.stderr.take() {
            pipe_reader(stderr, Arc::clone(&self.runtime_log), "stderr");
        }
    }

    pub(super) fn poll_kernel_exit(&mut self) {
        let Some(child) = self.kernel.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.kernel = None;
                self.error(&format!(
                    "sing-box 已异常退出，退出状态：{}。请查看日志获取详细信息。",
                    format_exit_status(status)
                ));
            }
            Ok(None) => {}
            Err(err) => {
                self.kernel = None;
                self.error(&format!("检查 sing-box 运行状态失败：{err}"));
            }
        }
    }

    fn kernel_start_error(&mut self, message: &str) {
        self.error(&format!("内核启动失败：{message}"));
    }

    fn kernel_start_info(&mut self, message: &str) {
        self.state = AppState::Error;
        self.update_menu();
        self.log(message);
        show_info("SingBoost", message);
    }
}

fn format_exit_status(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("退出码 {code}"),
        None => status.to_string(),
    }
}
