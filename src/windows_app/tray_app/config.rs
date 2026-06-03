use singboost::{AppConfig, AppPaths};

pub(super) fn write_config(paths: &AppPaths, config: &AppConfig) -> std::io::Result<()> {
    let mut text = format!(
        "[app]\nrun_as_admin = {}\n\n[sing_box]\nstart_command = {}\n",
        config.run_as_admin,
        toml::Value::String(config.start_command.clone())
    );
    if let Some(subscription) = &config.subscription {
        text.push_str("\n[subscription]\n");
        if let Some(url) = &subscription.url {
            text.push_str(&format!("url = {}\n", toml::Value::String(url.clone())));
        }
        if let Some(target) = &subscription.target {
            text.push_str(&format!(
                "target = {}\n",
                toml::Value::String(target.clone())
            ));
        }
    }
    std::fs::write(paths.config_toml(), text)
}
