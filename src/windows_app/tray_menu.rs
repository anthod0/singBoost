use std::error::Error;
use tray_icon::Icon;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};

pub(crate) const START_STOP_ID: &str = "start_stop";
pub(crate) const RESTART_ID: &str = "restart";
pub(crate) const OPEN_UI_ID: &str = "open_ui";
pub(crate) const LOG_ID: &str = "log";
pub(crate) const ADMIN_ID: &str = "admin";
pub(crate) const AUTOSTART_ID: &str = "autostart";
pub(crate) const ABOUT_ID: &str = "about";
pub(crate) const EXIT_ID: &str = "exit";

pub(crate) struct TrayMenu {
    pub(crate) start_stop: MenuItem,
    pub(crate) restart: MenuItem,
    pub(crate) open_ui: MenuItem,
    pub(crate) admin: CheckMenuItem,
    pub(crate) autostart: CheckMenuItem,
}

pub(crate) fn create_menu(run_as_admin: bool, autostart: bool) -> (Menu, TrayMenu) {
    let menu = Menu::new();
    let start_stop = MenuItem::with_id(START_STOP_ID, "启动", true, None);
    let restart = MenuItem::with_id(RESTART_ID, "重启", false, None);
    let open_ui = MenuItem::with_id(OPEN_UI_ID, "打开 UI", true, None);
    let log = MenuItem::with_id(LOG_ID, "日志", true, None);
    let admin = CheckMenuItem::with_id(ADMIN_ID, "以管理员身份运行", true, run_as_admin, None);
    let autostart = CheckMenuItem::with_id(AUTOSTART_ID, "开机自启", true, autostart, None);
    let about = MenuItem::with_id(ABOUT_ID, "关于", true, None);
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
        &about,
        &exit,
    ]);
    (
        menu,
        TrayMenu {
            start_stop,
            restart,
            open_ui,
            admin,
            autostart,
        },
    )
}

pub(crate) fn create_icon() -> Result<Icon, Box<dyn Error>> {
    const TRAY_ICON: &[u8] = include_bytes!("../../assets/tray-icon.rgba");

    Ok(Icon::from_rgba(TRAY_ICON.to_vec(), 32, 32)?)
}
