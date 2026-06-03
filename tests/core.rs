use std::path::PathBuf;

use singboost::{
    AppConfig, AppPaths, ConfigError, KernelCommand, PreflightError, RuntimeLog,
    ensure_config_file, load_config, resolve_web_ui_url, spawn_command_line,
    validate_preflight_files,
};

#[test]
fn windows_binary_uses_gui_subsystem_to_avoid_console_window() {
    let main_rs = std::fs::read_to_string("src/main.rs").unwrap();

    assert!(
        main_rs.contains("windows_subsystem = \"windows\""),
        "Windows builds must use the GUI subsystem so launching SingBoost does not open a console window"
    );
}

#[test]
fn app_paths_are_derived_from_app_dir() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));

    assert_eq!(paths.app_dir(), PathBuf::from(r"D:\Program Files\sing-box"));
    assert_eq!(
        paths.sing_box_exe(),
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        paths.config_json(),
        PathBuf::from(r"D:\Program Files\sing-box\config.json")
    );
    assert_eq!(
        paths.logs_dir(),
        PathBuf::from(r"D:\Program Files\sing-box\logs")
    );
    assert_eq!(
        paths.config_toml(),
        PathBuf::from(r"D:\Program Files\sing-box\singboost.toml")
    );
    assert_eq!(
        paths.runtime_log(),
        PathBuf::from(r"D:\Program Files\sing-box\logs\singboost-runtime.log")
    );
}

#[test]
fn sing_box_check_command_matches_spec() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let command = KernelCommand::check(&paths);

    assert_eq!(
        command.program,
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        command.args,
        vec![
            "check".to_string(),
            "-D".to_string(),
            r"D:\Program Files\sing-box".to_string(),
            "-c".to_string(),
            r"D:\Program Files\sing-box\config.json".to_string(),
        ]
    );
}

#[test]
fn sing_box_run_command_matches_spec() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let config = AppConfig::default_for_app_dir(&paths.app_dir());
    let command = KernelCommand::run(&paths, &config);

    assert_eq!(
        command.program,
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        command.args,
        vec![
            "-D".to_string(),
            r"D:\Program Files\sing-box".to_string(),
            "-c".to_string(),
            r"D:\Program Files\sing-box\config.json".to_string(),
            "run".to_string(),
        ]
    );
}

#[test]
fn creates_default_config_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());

    ensure_config_file(&paths).unwrap();

    let content = std::fs::read_to_string(paths.config_toml()).unwrap();
    assert!(content.contains("[app]"));
    assert!(content.contains("run_as_admin = false"));
    assert!(content.contains("[sing_box]"));
    assert!(content.contains(
        "start_command = 'sing-box.exe -D \"<app_dir>\" -c \"<app_dir>\\config.json\" run'"
    ));
}

#[test]
fn loads_config_without_fallback_when_field_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        "[app]\nrun_as_admin = false\n[sing_box]\n",
    )
    .unwrap();

    let error = load_config(&paths).unwrap_err();

    assert!(matches!(error, ConfigError::MissingStartCommand));
}

#[test]
fn rejects_empty_start_command() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        "[app]\nrun_as_admin = false\n[sing_box]\nstart_command = '   '\n",
    )
    .unwrap();

    let error = load_config(&paths).unwrap_err();

    assert!(matches!(error, ConfigError::EmptyStartCommand));
}

#[test]
fn expands_app_dir_placeholder_in_start_command() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let config = AppConfig::default_for_app_dir(&paths.app_dir());

    assert_eq!(
        config.expanded_start_command(&paths.app_dir()),
        r#"sing-box.exe -D "D:\Program Files\sing-box" -c "D:\Program Files\sing-box\config.json" run"#
    );
}

#[test]
fn resolves_web_ui_url_from_sing_box_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "experimental": {
                "clash_api": {
                    "external_controller": "0.0.0.0:20123",
                    "external_ui": "ui"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://127.0.0.1:20123/ui/");
}

#[test]
fn resolves_web_ui_url_from_non_loopback_controller() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "experimental": {
                "clash_api": {
                    "external_controller": "192.168.1.10:20123"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://192.168.1.10:20123/ui/");
}

#[test]
fn resolves_web_ui_url_from_port_only_controller() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "experimental": {
                "clash_api": {
                    "external_controller": ":20123"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://127.0.0.1:20123/ui/");
}

#[test]
fn runtime_log_path_is_fixed_and_recreated() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::create_dir_all(paths.logs_dir()).unwrap();
    std::fs::write(paths.runtime_log(), "old contents").unwrap();

    let mut log = RuntimeLog::recreate(&paths).unwrap();
    log.append_event("started").unwrap();

    let content = std::fs::read_to_string(paths.runtime_log()).unwrap();
    assert!(!content.contains("old contents"));
    assert!(content.contains("started"));
}

#[test]
fn parses_quoted_windows_command_line() {
    let app_dir = PathBuf::from(r"D:\Program Files\sing-box");

    let command = spawn_command_line(
        r#"sing-box.exe -D "D:\Program Files\sing-box" -c "D:\Program Files\sing-box\config.json" run"#,
        &app_dir,
    )
    .unwrap();

    assert_eq!(
        command.program,
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        command.args,
        vec![
            "-D".to_string(),
            r"D:\Program Files\sing-box".to_string(),
            "-c".to_string(),
            r"D:\Program Files\sing-box\config.json".to_string(),
            "run".to_string(),
        ]
    );
}

#[test]
fn preflight_reports_missing_sing_box_before_starting() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.config_json(), "{}").unwrap();

    let error = validate_preflight_files(&paths).unwrap_err();

    assert!(matches!(error, PreflightError::MissingSingBox(_)));
}

#[test]
fn preflight_recreates_runtime_log_when_required_files_exist() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.sing_box_exe(), "fake exe").unwrap();
    std::fs::write(paths.config_json(), "{}").unwrap();

    validate_preflight_files(&paths).unwrap();

    assert!(paths.logs_dir().exists());
    assert_eq!(std::fs::read_to_string(paths.runtime_log()).unwrap(), "");
}
