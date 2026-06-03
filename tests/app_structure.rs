fn read_tray_app_sources() -> String {
    [
        "src/windows_app/tray_app.rs",
        "src/windows_app/tray_app/config.rs",
        "src/windows_app/tray_app/kernel.rs",
        "src/windows_app/tray_app/menu_actions.rs",
        "src/windows_app/tray_app/ui_state.rs",
    ]
    .into_iter()
    .map(|path| std::fs::read_to_string(path).unwrap())
    .collect::<Vec<_>>()
    .join("\n")
}

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
    let tray_app = read_tray_app_sources();
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
    let tray_app = read_tray_app_sources();

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
fn kernel_start_errors_use_chinese_user_facing_prefix() {
    let command_rs = std::fs::read_to_string("src/core/command.rs").unwrap();
    let tray_app = read_tray_app_sources();

    assert!(
        !command_rs.contains("expect(\"validated start command\")"),
        "Invalid sing_box.start_command must be returned to the tray app instead of panicking"
    );
    assert!(
        tray_app.contains("内核启动失败："),
        "Kernel startup failures should use a unified Chinese user-facing popup prefix"
    );
    assert!(
        tray_app.contains("启动命令无效"),
        "Invalid start_command should be classified as a kernel startup failure"
    );
}

#[test]
fn kernel_unexpected_exit_is_detected_and_reported_to_users() {
    let tray_app = read_tray_app_sources();

    assert!(
        tray_app.contains("poll_kernel_exit"),
        "The tray event loop should detect a sing-box process that exits after successful spawn"
    );
    assert!(
        tray_app.contains("Duration::from_secs(1)"),
        "Running sing-box should be polled at a conservative one-second interval"
    );
    assert!(
        tray_app.contains("ControlFlow::Wait") && tray_app.contains("self.kernel.is_some()"),
        "When sing-box is not running, the tray app should wait for menu events instead of polling"
    );
    assert!(
        tray_app.contains("sing-box 已异常退出"),
        "Unexpected sing-box exits should show a Chinese user-facing popup"
    );
}

#[test]
fn admin_toggle_reports_autostart_sync_failures() {
    let tray_app = read_tray_app_sources();

    assert!(
        tray_app.contains("管理员运行配置已更新，但同步开机自启任务失败"),
        "When admin mode changes, failures to update the existing scheduled task must not be ignored"
    );
    assert!(
        !tray_app.contains("let _ = set_autostart(&self.paths, self.config.run_as_admin);"),
        "Autostart synchronization errors after toggling admin mode should not be discarded"
    );
}

#[test]
fn startup_state_prevents_duplicate_start_clicks() {
    let core_mod = std::fs::read_to_string("src/core/mod.rs").unwrap();
    let tray_app = read_tray_app_sources();

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
