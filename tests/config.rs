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
    assert!(content.contains("# timeout_secs = 30"));
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
            "[subscription]\nurl = 'https://example.com/config.json'\ntarget = 'remote.json'\ntimeout_secs = 10\n",
        ),
    )
    .unwrap();

    let config = load_config(&paths).unwrap();

    assert_eq!(
        config.subscription,
        Some(SubscriptionConfig {
            url: Some("https://example.com/config.json".to_string()),
            target: Some("remote.json".to_string()),
            timeout_secs: Some(10),
            username: None,
            password: None,
        })
    );
}

#[test]
fn loads_basic_auth_credentials_without_trimming() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        concat!(
            "[sing_box]\nstart_command = 'sing-box run'\n",
            "[subscription]\nusername = ' user '\npassword = ' p@ss:word '\n",
        ),
    )
    .unwrap();

    let subscription = load_config(&paths).unwrap().subscription.unwrap();
    assert_eq!(subscription.username.as_deref(), Some(" user "));
    assert_eq!(subscription.password.as_deref(), Some(" p@ss:word "));
}

#[test]
fn rejects_incomplete_or_invalid_basic_auth_credentials() {
    for credentials in [
        "username = 'user'",
        "password = 'secret'",
        "username = ''\npassword = 'secret'",
        "username = 'user:name'\npassword = 'secret'",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(temp.path().to_path_buf());
        std::fs::write(
            paths.config_toml(),
            format!("[sing_box]\nstart_command = 'sing-box run'\n[subscription]\n{credentials}\n"),
        )
        .unwrap();
        assert!(matches!(
            load_config(&paths),
            Err(ConfigError::InvalidSubscriptionBasicAuth)
        ));
    }
}

#[test]
fn rejects_subscription_timeout_out_of_range() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        concat!(
            "[sing_box]\nstart_command = 'sing-box.exe -D . -c config.json run'\n\n",
            "[subscription]\nurl = 'https://example.com/config.json'\ntarget = 'remote.json'\ntimeout_secs = 0\n",
        ),
    )
    .unwrap();

    let error = load_config(&paths).unwrap_err();

    assert!(matches!(error, ConfigError::InvalidSubscriptionTimeout(0)));
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
        "# Managed by SingBoost. Do not edit manually.\nrun_as_admin = false\n"
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
        std::fs::read_to_string(paths.state_toml()).unwrap(),
        "# Managed by SingBoost. Do not edit manually.\nrun_as_admin = false\n"
    );
    assert_eq!(
        load_state_config(&paths).unwrap(),
        AppStateConfig {
            run_as_admin: false
        }
    );
}
