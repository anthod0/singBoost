mod app_update;
mod kernel;
mod kernel_update;
mod menu_actions;
mod ui_state;

use crate::windows_app::autostart::autostart_enabled;
use crate::windows_app::tray_menu::{TrayMenu, create_icon, create_menu};
use singboost::core::app_update::{AppUpdate, PreparedAppUpdate};
use singboost::{
    AppConfig, AppPaths, AppState, AppStateConfig, KernelUpdate, PreparedKernelUpdate, RuntimeLog,
};
use std::error::Error;
use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tray_icon::menu::MenuEvent;
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

enum UserEvent {
    Menu(MenuEvent),
    TrayIcon(TrayIconEvent),
    SubscriptionDownloadFinished(Result<PathBuf, String>),
    KernelUpdateChecked(Result<KernelUpdate, String>),
    KernelUpdatePrepared(Result<PreparedKernelUpdate, String>),
    AppUpdateChecked(Result<AppUpdate, String>),
    AppUpdatePrepared(Result<PreparedAppUpdate, String>),
}

pub(crate) struct TrayApp {
    paths: AppPaths,
    config: AppConfig,
    state_config: AppStateConfig,
    state: AppState,
    runtime_log: Arc<Mutex<RuntimeLog>>,
    kernel: Option<Child>,
    log_windows: Vec<Child>,
    subscription_downloading: bool,
    kernel_updating: bool,
    app_updating: bool,
    menu: TrayMenu,
    _tray: Option<TrayIcon>,
}

impl TrayApp {
    pub(crate) fn new(
        paths: AppPaths,
        config: AppConfig,
        state_config: AppStateConfig,
    ) -> Result<Self, Box<dyn Error>> {
        let mut runtime_log = RuntimeLog::recreate(&paths)?;
        runtime_log.append_event("SingBoost started")?;
        let runtime_log = Arc::new(Mutex::new(runtime_log));

        let (menu, tray_menu) = create_menu(state_config.run_as_admin, autostart_enabled());
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .with_tooltip("SingBoost")
            .with_icon(create_icon()?)
            .build()?;

        let app = Self {
            paths,
            config,
            state_config,
            state: AppState::Stopped,
            runtime_log,
            kernel: None,
            log_windows: Vec::new(),
            subscription_downloading: false,
            kernel_updating: false,
            app_updating: false,
            menu: tray_menu,
            _tray: Some(tray),
        };
        app.update_menu();
        Ok(app)
    }

    pub(crate) fn run(
        mut self,
        mut startup: Option<crate::windows_app::app_update::UpdateStartup>,
    ) -> ! {
        let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();
        let menu_proxy = proxy.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = menu_proxy.send_event(UserEvent::Menu(event));
        }));
        let tray_proxy = proxy.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            let _ = tray_proxy.send_event(UserEvent::TrayIcon(event));
        }));

        event_loop.run(move |event, _, control_flow| {
            if self.kernel.is_some() {
                *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_secs(1));
            } else {
                *control_flow = ControlFlow::Wait;
            }
            match event {
                Event::NewEvents(StartCause::Init) => {
                    if let Some(startup) = startup.take() {
                        if let Err(error) = startup.acknowledge() {
                            self.log(&format!(
                                "SingBoost startup acknowledgement failed: {error}"
                            ));
                            self.exit();
                        }
                    }
                    self.start_kernel();
                }
                Event::UserEvent(UserEvent::Menu(event)) => {
                    self.handle_menu(event.id().clone(), proxy.clone())
                }
                Event::UserEvent(UserEvent::TrayIcon(event)) => self.handle_tray_icon(event),
                Event::UserEvent(UserEvent::SubscriptionDownloadFinished(result)) => {
                    self.finish_subscription_download(result)
                }
                Event::UserEvent(UserEvent::KernelUpdateChecked(result)) => {
                    self.finish_kernel_update_check(result, proxy.clone())
                }
                Event::UserEvent(UserEvent::KernelUpdatePrepared(result)) => {
                    self.finish_kernel_update_prepare(result)
                }
                Event::UserEvent(UserEvent::AppUpdateChecked(result)) => {
                    self.finish_app_update_check(result, proxy.clone())
                }
                Event::UserEvent(UserEvent::AppUpdatePrepared(result)) => {
                    self.finish_app_update_prepare(result)
                }
                Event::MainEventsCleared => self.poll_kernel_exit(),
                _ => {}
            }
        });
    }

    fn handle_tray_icon(&mut self, event: TrayIconEvent) {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            if self.state == AppState::Running {
                self.open_web_ui();
            }
        }
    }
}
