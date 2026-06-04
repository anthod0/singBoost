use crate::support::read_tray_app_sources;

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
    let subscription_rs = std::fs::read_to_string("src/core/subscription.rs").unwrap();

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
    assert!(
        subscription_rs.contains("CREATE_NO_WINDOW") && subscription_rs.contains("creation_flags"),
        "subscription PowerShell downloads should not flash console windows"
    );
}

#[test]
fn windows_elevation_relaunches_the_current_executable_name() {
    let elevation_rs = std::fs::read_to_string("src/windows_app/elevation.rs").unwrap();

    assert!(
        elevation_rs.contains("current_exe"),
        "UAC relaunch must use the current executable path because release assets may be renamed by users or CI"
    );
    assert!(
        !elevation_rs.contains("join(\"singboost.exe\")"),
        "UAC relaunch must not hard-code singboost.exe; v0.3.1 release asset is named singboost-windows-x86_64.exe"
    );
}

#[test]
fn windows_app_rejects_duplicate_instances_after_elevation_handoff() {
    let windows_mod = std::fs::read_to_string("src/windows_app/mod.rs").unwrap();
    let single_instance =
        std::fs::read_to_string("src/windows_app/single_instance.rs").unwrap_or_default();

    let elevation_check = "if state_config.run_as_admin && !elevation::is_elevated()";
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
