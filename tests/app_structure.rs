#[test]
fn windows_binary_uses_gui_subsystem_to_avoid_console_window() {
    let main_rs = std::fs::read_to_string("src/main.rs").unwrap();

    assert!(
        main_rs.contains("windows_subsystem = \"windows\""),
        "Windows builds must use the GUI subsystem so launching SingBoost does not open a console window"
    );
}

#[test]
fn open_ui_menu_item_follows_kernel_running_state() {
    let tray_menu = std::fs::read_to_string("src/windows_app/tray_menu.rs").unwrap();
    let tray_app = std::fs::read_to_string("src/windows_app/tray_app.rs").unwrap();

    assert!(
        tray_menu.contains("open_ui: MenuItem"),
        "TrayMenu must keep an Open UI handle so its enabled state can follow the kernel state"
    );
    assert!(
        tray_app.contains("self.menu.open_ui.set_enabled(true)"),
        "Open UI should be enabled while the kernel is running"
    );
    assert!(
        tray_app.contains("self.menu.open_ui.set_enabled(false)"),
        "Open UI should be disabled while the kernel is stopped or in error"
    );
}

#[test]
fn source_is_split_by_responsibility() {
    for path in [
        "src/core/paths.rs",
        "src/core/config.rs",
        "src/core/command.rs",
        "src/core/preflight.rs",
        "src/core/runtime_log.rs",
        "src/core/web_ui.rs",
        "src/windows_app/tray_app.rs",
        "src/windows_app/tray_menu.rs",
        "src/windows_app/process.rs",
        "src/windows_app/autostart.rs",
        "src/windows_app/elevation.rs",
        "src/windows_app/error_dialog.rs",
    ] {
        assert!(std::path::Path::new(path).exists(), "missing {path}");
    }
}

#[test]
fn tray_icon_asset_is_a_32px_rgba_image() {
    let icon = std::fs::read("assets/tray-icon.rgba").unwrap();

    assert_eq!(icon.len(), 32 * 32 * 4);
    assert!(icon.chunks_exact(4).any(|pixel| pixel[3] > 0));
}
