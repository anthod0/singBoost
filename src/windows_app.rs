#![cfg(windows)]

use singboost::{
    AppConfig, AppPaths, AppState, KernelCommand, RuntimeLog, ensure_config_file, load_config,
    resolve_web_ui_url, validate_preflight_files,
};
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW, SW_SHOWNORMAL};
use windows::core::{HSTRING, PCWSTR};

const START_STOP_ID: &str = "start_stop";
const RESTART_ID: &str = "restart";
const OPEN_UI_ID: &str = "open_ui";
const LOG_ID: &str = "log";
const ADMIN_ID: &str = "admin";
const AUTOSTART_ID: &str = "autostart";
const EXIT_ID: &str = "exit";
const TASK_NAME: &str = "SingBoost";

pub fn run() -> Result<(), Box<dyn Error>> {
    let paths = AppPaths::from_current_exe()?;
    ensure_config_file(&paths)?;
    let config = load_config(&paths)?;

    if config.run_as_admin && !is_elevated() {
        relaunch_elevated(&paths)?;
        return Ok(());
    }

    let app = TrayApp::new(paths, config)?;
    app.run();
}

struct TrayApp {
    paths: AppPaths,
    config: AppConfig,
    state: AppState,
    runtime_log: Arc<Mutex<RuntimeLog>>,
    kernel: Option<Child>,
    log_windows: Vec<Child>,
    menu: TrayMenu,
    _tray: Option<TrayIcon>,
}

struct TrayMenu {
    start_stop: MenuItem,
    restart: MenuItem,
    admin: CheckMenuItem,
    autostart: CheckMenuItem,
}

impl TrayApp {
    fn new(paths: AppPaths, config: AppConfig) -> Result<Self, Box<dyn Error>> {
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

    fn run(mut self) -> ! {
        enum UserEvent {
            Menu(MenuEvent),
        }
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = proxy.send_event(UserEvent::Menu(event));
        }));

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::NewEvents(StartCause::Init) => {}
                Event::UserEvent(UserEvent::Menu(event)) => self.handle_menu(event.id().clone()),
                _ => {}
            }
        });
    }

    fn handle_menu(&mut self, id: MenuId) {
        match id.as_ref() {
            START_STOP_ID => match self.state {
                AppState::Running => self.stop_kernel(),
                AppState::Stopped | AppState::Error => self.start_kernel(),
            },
            RESTART_ID => self.restart_kernel(),
            OPEN_UI_ID => match resolve_web_ui_url(&self.paths) {
                Ok(url) => {
                    if let Err(err) = open::that(&url) {
                        self.error(&format!("failed to open UI {url}: {err}"));
                    }
                }
                Err(err) => self.error(&format!("failed to resolve UI URL: {err}")),
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
        if matches!(self.state, AppState::Running) {
            return;
        }
        if let Err(err) = validate_preflight_files(&self.paths) {
            self.error(&format!("preflight failed: {err}"));
            return;
        }
        if let Err(err) = self.run_check() {
            self.error(&format!("sing-box check failed: {err}"));
            return;
        }

        let command = KernelCommand::run(&self.paths, &self.config);
        match Command::new(&command.program)
            .args(&command.args)
            .current_dir(self.paths.app_dir())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                self.pipe_child_logs(&mut child);
                self.log("sing-box started");
                self.kernel = Some(child);
                self.state = AppState::Running;
            }
            Err(err) => self.error(&format!("failed to start sing-box: {err}")),
        }
    }

    fn run_check(&mut self) -> Result<(), String> {
        let command = KernelCommand::check(&self.paths);
        let output = Command::new(&command.program)
            .args(&command.args)
            .current_dir(self.paths.app_dir())
            .output()
            .map_err(|err| err.to_string())?;
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
            Err(err) => self.error(&format!("failed to open log window: {err}")),
        }
    }

    fn toggle_admin(&mut self) {
        let enabled = !self.config.run_as_admin;
        self.config.run_as_admin = enabled;
        if let Err(err) = write_config(&self.paths, &self.config) {
            self.error(&format!("failed to write config: {err}"));
            return;
        }
        if enabled && !is_elevated() {
            let paths = self.paths.clone();
            self.exit_after(|| relaunch_elevated(&paths));
        }
        if autostart_enabled() {
            let _ = set_autostart(&self.paths, self.config.run_as_admin);
        }
    }

    fn toggle_autostart(&mut self) {
        if autostart_enabled() {
            if let Err(err) = remove_autostart() {
                self.error(&format!("failed to remove autostart: {err}"));
            }
        } else if let Err(err) = set_autostart(&self.paths, self.config.run_as_admin) {
            self.error(&format!("failed to enable autostart: {err}"));
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
            self.error(&format!("failed before exit: {err}"));
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
                self.menu.restart.set_enabled(true);
            }
            AppState::Stopped | AppState::Error => {
                self.menu.start_stop.set_text("启动");
                self.menu.restart.set_enabled(false);
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

    fn error(&mut self, message: &str) {
        self.state = AppState::Error;
        self.log(message);
        show_error(message);
    }
}

fn create_menu(run_as_admin: bool, autostart: bool) -> (Menu, TrayMenu) {
    let menu = Menu::new();
    let start_stop = MenuItem::with_id(START_STOP_ID, "启动", true, None);
    let restart = MenuItem::with_id(RESTART_ID, "重启", false, None);
    let open_ui = MenuItem::with_id(OPEN_UI_ID, "打开 UI", true, None);
    let log = MenuItem::with_id(LOG_ID, "日志", true, None);
    let admin = CheckMenuItem::with_id(ADMIN_ID, "以管理员身份运行", true, run_as_admin, None);
    let autostart = CheckMenuItem::with_id(AUTOSTART_ID, "开机自启", true, autostart, None);
    let exit = MenuItem::with_id(EXIT_ID, "退出", true, None);
    let separator = PredefinedMenuItem::separator();
    let _ = menu.append_items(&[
        &start_stop,
        &restart,
        &open_ui,
        &log,
        &separator,
        &admin,
        &autostart,
        &separator,
        &exit,
    ]);
    (
        menu,
        TrayMenu {
            start_stop,
            restart,
            admin,
            autostart,
        },
    )
}

fn create_icon() -> Result<Icon, Box<dyn Error>> {
    let mut rgba = Vec::with_capacity(32 * 32 * 4);
    for y in 0..32 {
        for x in 0..32 {
            let edge = x < 3 || y < 3 || x > 28 || y > 28;
            let (r, g, b, a) = if edge {
                (30, 144, 255, 255)
            } else {
                (20, 20, 20, 255)
            };
            rgba.extend([r, g, b, a]);
        }
    }
    Ok(Icon::from_rgba(rgba, 32, 32)?)
}

fn pipe_reader<R>(reader: R, log: Arc<Mutex<RuntimeLog>>, label: &'static str)
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            if let Ok(mut log) = log.lock() {
                let _ = log.append_event(format!("{label}: {line}"));
            }
        }
    });
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
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

fn autostart_enabled() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn set_autostart(paths: &AppPaths, highest: bool) -> Result<(), Box<dyn Error>> {
    let exe = paths.app_dir().join("singboost.exe");
    let mut args = vec![
        "/Create".to_string(),
        "/F".to_string(),
        "/TN".to_string(),
        TASK_NAME.to_string(),
        "/SC".to_string(),
        "ONLOGON".to_string(),
        "/TR".to_string(),
        format!("\"{}\"", exe.display()),
    ];
    if highest {
        args.extend(["/RL".to_string(), "HIGHEST".to_string()]);
    }
    let status = Command::new("schtasks").args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks exited with {status}").into())
    }
}

fn remove_autostart() -> Result<(), Box<dyn Error>> {
    let status = Command::new("schtasks")
        .args(["/Delete", "/F", "/TN", TASK_NAME])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks exited with {status}").into())
    }
}

fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

fn relaunch_elevated(paths: &AppPaths) -> Result<(), Box<dyn Error>> {
    let exe = HSTRING::from(
        paths
            .app_dir()
            .join("singboost.exe")
            .to_string_lossy()
            .as_ref(),
    );
    let verb = HSTRING::from("runas");
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("ShellExecuteW runas failed".into())
    } else {
        Ok(())
    }
}

pub(crate) fn show_error(message: &str) {
    let title = HSTRING::from("SingBoost");
    let text = HSTRING::from(message);
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}
