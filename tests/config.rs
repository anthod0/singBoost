use std::path::PathBuf;

use singboost::{
    AppConfig, AppPaths, ConfigError, SubscriptionConfig, ensure_config_file, load_config,
    save_subscription_url,
};

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
    assert!(!content.contains("[subscription]"));
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

#[test]
fn loads_optional_subscription_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        concat!(
            "[app]\nrun_as_admin = false\n\n",
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
        "[app]\nrun_as_admin = false\n[sing_box]\nstart_command = 'sing-box.exe -D . -c config.json run'\n",
    )
    .unwrap();

    let config = load_config(&paths).unwrap();

    assert_eq!(config.subscription, None);
}

#[test]
fn saves_subscription_url_adds_default_target_when_missing() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        concat!(
            "[app]\nrun_as_admin = false\n\n",
            "[sing_box]\nstart_command = 'sing-box.exe -D . -c config.json run'\n",
        ),
    )
    .unwrap();

    save_subscription_url(&paths, "https://example.com/new.json").unwrap();
    let config = load_config(&paths).unwrap();

    assert_eq!(
        config.subscription,
        Some(SubscriptionConfig {
            url: Some("https://example.com/new.json".to_string()),
            target: Some("config.json".to_string()),
        })
    );
}

#[test]
fn saves_subscription_url_without_losing_target_or_app_settings() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_toml(),
        concat!(
            "[app]\nrun_as_admin = true\n\n",
            "[sing_box]\nstart_command = 'sing-box.exe -D . -c custom.json run'\n\n",
            "[subscription]\ntarget = 'config.json'\n",
        ),
    )
    .unwrap();

    save_subscription_url(&paths, "https://example.com/new.json").unwrap();
    let config = load_config(&paths).unwrap();

    assert!(config.run_as_admin);
    assert_eq!(config.start_command, "sing-box.exe -D . -c custom.json run");
    assert_eq!(
        config.subscription,
        Some(SubscriptionConfig {
            url: Some("https://example.com/new.json".to_string()),
            target: Some("config.json".to_string()),
        })
    );
}
