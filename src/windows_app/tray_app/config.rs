use singboost::{AppConfig, AppPaths};

pub(super) fn write_config(paths: &AppPaths, config: &AppConfig) -> std::io::Result<()> {
    let escaped = config
        .start_command
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    std::fs::write(
        paths.config_toml(),
        format!(
            "[app]\nrun_as_admin = {}\n\n[sing_box]\nstart_command = \"{}\"\n",
            config.run_as_admin, escaped
        ),
    )
}
