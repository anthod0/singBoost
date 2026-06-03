use crate::windows_app::process::hide_window;
use singboost::AppPaths;
use std::error::Error;
use std::process::{Command, Stdio};

const TASK_NAME: &str = "SingBoost";

pub(crate) fn autostart_enabled() -> bool {
    let mut command = Command::new("schtasks");
    hide_window(
        command
            .args(["/Query", "/TN", TASK_NAME])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .status()
    .map(|status| status.success())
    .unwrap_or(false)
}

pub(crate) fn set_autostart(paths: &AppPaths, highest: bool) -> Result<(), Box<dyn Error>> {
    let exe = paths.app_dir().join("singboost.exe");
    let mut args = vec![
        "/Create".to_string(),
        "/F".to_string(),
        "/TN".to_string(),
        TASK_NAME.to_string(),
        "/SC".to_string(),
        "ONLOGON".to_string(),
        "/TR".to_string(),
        format!("\"{}\"", exe.display()),
    ];
    if highest {
        args.extend(["/RL".to_string(), "HIGHEST".to_string()]);
    }
    let mut command = Command::new("schtasks");
    let status = hide_window(command.args(args)).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks exited with {status}").into())
    }
}

pub(crate) fn remove_autostart() -> Result<(), Box<dyn Error>> {
    let mut command = Command::new("schtasks");
    let status = hide_window(command.args(["/Delete", "/F", "/TN", TASK_NAME])).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks exited with {status}").into())
    }
}
