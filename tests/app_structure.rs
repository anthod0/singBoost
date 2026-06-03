#[test]
fn windows_binary_uses_gui_subsystem_to_avoid_console_window() {
    let main_rs = std::fs::read_to_string("src/main.rs").unwrap();

    assert!(
        main_rs.contains("windows_subsystem = \"windows\""),
        "Windows builds must use the GUI subsystem so launching SingBoost does not open a console window"
    );
}

#[test]
fn windows_child_processes_are_hidden_unless_user_opens_log_window() {
    let process_rs = std::fs::read_to_string("src/windows_app/process.rs").unwrap();
    let tray_app = std::fs::read_to_string("src/windows_app/tray_app.rs").unwrap();
    let autostart_rs = std::fs::read_to_string("src/windows_app/autostart.rs").unwrap();

    assert!(
        process_rs.contains("CREATE_NO_WINDOW") && process_rs.contains("hide_window"),
        "Windows console child processes must be spawned with CREATE_NO_WINDOW"
    );
    assert!(
        tray_app.matches("hide_window(").count() >= 2,
        "sing-box check and run commands should not flash console windows"
    );
    assert!(
        autostart_rs.contains("hide_window("),
        "schtasks queries/updates should not flash console windows"
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
fn startup_state_prevents_duplicate_start_clicks() {
    let core_mod = std::fs::read_to_string("src/core/mod.rs").unwrap();
    let tray_app = std::fs::read_to_string("src/windows_app/tray_app.rs").unwrap();

    assert!(
        core_mod.contains("Starting"),
        "AppState should include a Starting state for in-progress startup"
    );
    assert!(
        tray_app.contains("self.state = AppState::Starting"),
        "Starting state should be set before preflight/check/spawn work begins"
    );
    assert!(
        tray_app.contains("AppState::Starting =>")
            && tray_app.contains("self.menu.start_stop.set_text(\"启动中...\")")
            && tray_app.contains("self.menu.start_stop.set_enabled(false)"),
        "Tray menu should show and disable the start item while startup is in progress"
    );
    assert!(
        tray_app.contains("self.state = AppState::Running;\n                self.update_menu();")
            && tray_app.contains("self.state = AppState::Error;\n        self.update_menu();"),
        "Tray menu should be refreshed when startup finishes successfully or fails"
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
        "src/windows_app/single_instance.rs",
    ] {
        assert!(std::path::Path::new(path).exists(), "missing {path}");
    }
}

#[test]
fn windows_app_rejects_duplicate_instances_after_elevation_handoff() {
    let windows_mod = std::fs::read_to_string("src/windows_app/mod.rs").unwrap();
    let single_instance =
        std::fs::read_to_string("src/windows_app/single_instance.rs").unwrap_or_default();

    let elevation_check = "if config.run_as_admin && !elevation::is_elevated()";
    let single_instance_check = "single_instance::acquire()";
    assert!(
        windows_mod.contains("mod single_instance;"),
        "Windows app must include a single-instance guard module"
    );
    assert!(
        windows_mod.contains(single_instance_check),
        "Windows startup must acquire a single-instance guard"
    );
    assert!(
        windows_mod.find(elevation_check).unwrap()
            < windows_mod.find(single_instance_check).unwrap(),
        "Single-instance guard must be acquired after UAC relaunch handoff so the unelevated launcher does not block the elevated real instance"
    );
    assert!(
        single_instance.contains("CreateMutexW")
            && single_instance.contains("ERROR_ALREADY_EXISTS"),
        "Single-instance guard should use a Windows named mutex and treat an existing mutex as a duplicate instance"
    );
    assert!(
        single_instance.contains("ERROR_ACCESS_DENIED"),
        "Single-instance guard should treat access denied as an existing elevated instance"
    );
}

#[test]
fn tray_icon_asset_is_a_32px_rgba_image() {
    let icon = std::fs::read("assets/tray-icon.rgba").unwrap();

    assert_eq!(icon.len(), 32 * 32 * 4);
    assert!(icon.chunks_exact(4).any(|pixel| pixel[3] > 0));
}
