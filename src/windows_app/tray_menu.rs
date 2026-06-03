use std::error::Error;
use tray_icon::Icon;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};

pub(crate) const START_STOP_ID: &str = "start_stop";
pub(crate) const RESTART_ID: &str = "restart";
pub(crate) const OPEN_UI_ID: &str = "open_ui";
pub(crate) const LOG_ID: &str = "log";
pub(crate) const CONFIG_MENU_ID: &str = "config_menu";
pub(crate) const OPEN_CONFIG_ID: &str = "open_config";
pub(crate) const OPEN_APP_DIR_ID: &str = "open_app_dir";
pub(crate) const OPEN_SING_BOX_CONFIG_ID: &str = "open_sing_box_config";
pub(crate) const DOWNLOAD_REMOTE_CONFIG_ID: &str = "download_remote_config";
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
    let open_config = MenuItem::with_id(OPEN_CONFIG_ID, "打开配置文件", true, None);
    let open_app_dir = MenuItem::with_id(OPEN_APP_DIR_ID, "打开程序目录", true, None);
    let open_sing_box_config = MenuItem::with_id(
        OPEN_SING_BOX_CONFIG_ID,
        "打开 sing-box 配置文件",
        true,
        None,
    );
    let download_remote_config =
        MenuItem::with_id(DOWNLOAD_REMOTE_CONFIG_ID, "下载远程配置", true, None);
    let config_separator = PredefinedMenuItem::separator();
    let config_menu = Submenu::with_id_and_items(
        CONFIG_MENU_ID,
        "配置",
        true,
        &[
            &open_config,
            &open_app_dir,
            &open_sing_box_config,
            &config_separator,
            &download_remote_config,
        ],
    )
    .expect("create config submenu");
    let admin = CheckMenuItem::with_id(ADMIN_ID, "以管理员身份运行", true, run_as_admin, None);
    let autostart = CheckMenuItem::with_id(AUTOSTART_ID, "开机自启", true, autostart, None);
    let about = MenuItem::with_id(ABOUT_ID, "关于", true, None);
    let exit = MenuItem::with_id(EXIT_ID, "退出", true, None);
    let settings_separator = PredefinedMenuItem::separator();
    let exit_separator = PredefinedMenuItem::separator();
    let _ = menu.append_items(&[
        &start_stop,
        &restart,
        &open_ui,
        &log,
        &config_menu,
        &settings_separator,
        &admin,
        &autostart,
        &exit_separator,
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
