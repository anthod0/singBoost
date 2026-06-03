use std::path::PathBuf;

use singboost::{AppConfig, AppPaths, ConfigError, ensure_config_file, load_config};

#[test]
fn creates_default_config_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());

    ensure_config_file(&paths).unwrap();

    let content = std::fs::read_to_string(paths.config_toml()).unwrap();
    assert!(content.contains("[app]"));
    assert!(content.contains("run_as_admin = false"));
    assert!(content.contains("[sing_box]"));
    assert!(content.contains("start_command = 'sing-box.exe -D . -c config.json run'"));
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
fn default_start_command_uses_relative_paths() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let config = AppConfig::default_for_app_dir(&paths.app_dir());

    assert_eq!(config.start_command, "sing-box.exe -D . -c config.json run");
}
