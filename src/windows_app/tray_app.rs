mod config;
mod kernel;
mod menu_actions;
mod ui_state;

use crate::windows_app::autostart::autostart_enabled;
use crate::windows_app::tray_menu::{TrayMenu, create_icon, create_menu};
use singboost::{AppConfig, AppPaths, AppState, RuntimeLog};
use std::error::Error;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;
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
}
