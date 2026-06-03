use crate::support::read_tray_app_sources;

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
