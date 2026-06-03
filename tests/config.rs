use std::path::PathBuf;

use singboost::{
    AppConfig, AppPaths, AppStateConfig, ConfigError, SubscriptionConfig, ensure_config_file,
    ensure_state_file, load_config, load_state_config, save_state_config,
};

#[test]
fn creates_default_config_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());

    ensure_config_file(&paths).unwrap();

    let content = std::fs::read_to_string(paths.config_toml()).unwrap();
    assert!(!content.contains("[app]"));
    assert!(!content.contains("run_as_admin"));
    assert!(content.contains("[sing_box]"));
    assert!(content.contains("start_command = 'sing-box.exe -D . -c config.json run'"));
    assert!(content.contains("# 可选：下载远程完整 sing-box 配置。"));
    assert!(content.contains("# 在这里填写地址后，使用托盘菜单：配置 -> 下载远程配置"));
    assert!(content.contains("# [subscription]"));
    assert!(content.contains("# url = \"https://example.com/config.json\""));
    assert!(content.contains("# target = \"config.json\""));
    assert!(!content.contains("\n[subscription]"));
}

#[test]
fn loads_config_without_fallback_when_field_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.config_toml(), "[sing_box]\n").unwrap();

    let error = load_config(&paths).unwrap_err();

    assert!(matches!(error, ConfigError::MissingStartCommand));
}

#[test]
fn rejects_empty_start_command() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.config_toml(), "[sing_box]\nstart_command = '   '\n").unwrap();

    let error = load_config(&paths).unwrap_err();

    assert!(matches!(error, ConfigError::EmptyStartCommand));
}

#[test]
fn default_start_command_uses_relative_paths() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let config = AppConfig::default_for_app_dir(&paths.app_dir());

    assert_eq!(config.start_command, "sing-box.exe -D . -c config.json run");
}

#[test]
fn loads_optional_subscription_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        concat!(
            "[sing_box]\nstart_command = 'sing-box.exe -D . -c config.json run'\n\n",
            "[subscription]\nurl = 'https://example.com/config.json'\ntarget = 'remote.json'\n",
        ),
    )
    .unwrap();

    let config = load_config(&paths).unwrap();

    assert_eq!(
        config.subscription,
        Some(SubscriptionConfig {
            url: Some("https://example.com/config.json".to_string()),
            target: Some("remote.json".to_string()),
        })
    );
}

#[test]
fn missing_subscription_does_not_affect_startup() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        "[sing_box]\nstart_command = 'sing-box.exe -D . -c config.json run'\n",
    )
    .unwrap();

    let config = load_config(&paths).unwrap();

    assert_eq!(config.subscription, None);
}

#[test]
fn creates_default_state_config_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());

    ensure_state_file(&paths).unwrap();

    assert_eq!(
        std::fs::read_to_string(paths.state_toml()).unwrap(),
        "run_as_admin = false\n"
    );
}

#[test]
fn loads_and_saves_state_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.state_toml(), "run_as_admin = true\n").unwrap();

    assert_eq!(
        load_state_config(&paths).unwrap(),
        AppStateConfig { run_as_admin: true }
    );

    save_state_config(
        &paths,
        &AppStateConfig {
            run_as_admin: false,
        },
    )
    .unwrap();

    assert_eq!(
        load_state_config(&paths).unwrap(),
        AppStateConfig {
            run_as_admin: false
        }
    );
}
