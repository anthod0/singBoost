use crate::windows_app::autostart::{autostart_enabled, remove_autostart, set_autostart};
use crate::windows_app::elevation::{is_elevated, relaunch_elevated};
use crate::windows_app::error_dialog::show_error;
use crate::windows_app::process::{hide_window, pipe_reader, terminate_child};
use crate::windows_app::tray_menu::{
    ADMIN_ID, AUTOSTART_ID, EXIT_ID, LOG_ID, OPEN_UI_ID, RESTART_ID, START_STOP_ID, TrayMenu,
    create_icon, create_menu,
};
use singboost::{
    AppConfig, AppPaths, AppState, KernelCommand, RuntimeLog, resolve_web_ui_url,
    sing_box_tun_enabled, validate_preflight_files,
};
use std::error::Error;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{MenuEvent, MenuId};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub(crate) struct TrayApp {
    paths: AppPaths,
    config: AppConfig,
    state: AppState,
    runtime_log: Arc<Mutex<RuntimeLog>>,
    kernel: Option<Child>,
    log_windows: Vec<Child>,
    menu: TrayMenu,
    _tray: Option<TrayIcon>,
}

impl TrayApp {
    pub(crate) fn new(paths: AppPaths, config: AppConfig) -> Result<Self, Box<dyn Error>> {
        let mut runtime_log = RuntimeLog::recreate(&paths)?;
        runtime_log.append_event("SingBoost started")?;
        let runtime_log = Arc::new(Mutex::new(runtime_log));

        let (menu, tray_menu) = create_menu(config.run_as_admin, autostart_enabled());
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("SingBoost")
            .with_icon(create_icon()?)
            .build()?;

        let mut app = Self {
            paths,
            config,
            state: AppState::Stopped,
            runtime_log,
            kernel: None,
            log_windows: Vec::new(),
            menu: tray_menu,
            _tray: Some(tray),
        };
        app.update_menu();
        app.start_kernel();
        Ok(app)
    }

    pub(crate) fn run(mut self) -> ! {
        enum UserEvent {
            Menu(MenuEvent),
        }
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::Menu(event));
        }));

        event_loop.run(move |event, _, control_flow| {
            if self.kernel.is_some() {
                *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));
            } else {
                *control_flow = ControlFlow::Wait;
            }
            match event {
                Event::NewEvents(StartCause::Init) => {}
                Event::UserEvent(UserEvent::Menu(event)) => self.handle_menu(event.id().clone()),
                Event::MainEventsCleared => self.poll_kernel_exit(),
                _ => {}
            }
        });
    }

    fn handle_menu(&mut self, id: MenuId) {
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

    fn start_kernel(&mut self) {
        if matches!(self.state, AppState::Running | AppState::Starting) {
            return;
        }
        self.state = AppState::Starting;
        self.update_menu();

        if let Err(err) = validate_preflight_files(&self.paths) {
            self.kernel_start_error(&format!("启动前检查失败：{err}"));
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

    fn stop_kernel(&mut self) {
        if let Some(mut child) = self.kernel.take() {
            terminate_child(&mut child);
            self.log("sing-box stopped");
        }
        self.state = AppState::Stopped;
    }

    fn restart_kernel(&mut self) {
        self.stop_kernel();
        self.start_kernel();
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

    fn pipe_child_logs(&self, child: &mut Child) {
        if let Some(stdout) = child.stdout.take() {
            pipe_reader(stdout, Arc::clone(&self.runtime_log), "stdout");
        }
        if let Some(stderr) = child.stderr.take() {
            pipe_reader(stderr, Arc::clone(&self.runtime_log), "stderr");
        }
    }

    fn update_menu(&self) {
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
        self.menu.admin.set_checked(self.config.run_as_admin);
        self.menu.autostart.set_checked(autostart_enabled());
    }

    fn log(&self, message: &str) {
        if let Ok(mut log) = self.runtime_log.lock() {
            let _ = log.append_event(message);
        }
    }

    fn poll_kernel_exit(&mut self) {
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

    fn error(&mut self, message: &str) {
        self.state = AppState::Error;
        self.update_menu();
        self.log(message);
        show_error(message);
    }
}

fn format_exit_status(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("退出码 {code}"),
        None => status.to_string(),
    }
}

fn write_config(paths: &AppPaths, config: &AppConfig) -> std::io::Result<()> {
    let escaped = config
        .start_command
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    std::fs::write(
        paths.config_toml(),
        format!(
            "[app]\nrun_as_admin = {}\n\n[sing_box]\nstart_command = \"{}\"\n",
            config.run_as_admin, escaped
        ),
    )
}
