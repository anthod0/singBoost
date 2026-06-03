use std::path::PathBuf;

use singboost::{AppConfig, AppPaths, CommandLineError, KernelCommand, spawn_command_line};

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
    let command = KernelCommand::run(&paths, &config).unwrap();

    assert_eq!(
        command.program,
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        command.args,
        vec![
            "-D".to_string(),
            ".".to_string(),
            "-c".to_string(),
            "config.json".to_string(),
            "run".to_string(),
        ]
    );
}

#[test]
fn invalid_sing_box_start_command_returns_error() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let config = AppConfig {
        run_as_admin: false,
        start_command: r#""sing-box.exe -D . -c config.json run"#.to_string(),
    };

    let error = KernelCommand::run(&paths, &config).unwrap_err();

    assert_eq!(error, CommandLineError::UnterminatedQuote);
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
