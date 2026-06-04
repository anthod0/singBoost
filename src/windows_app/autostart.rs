use crate::windows_app::process::hide_window;
use singboost::{AppPaths, AppStateConfig};
use std::error::Error;
use std::process::{Command, Stdio};

const TASK_NAME: &str = "SingBoost";

pub(crate) fn autostart_enabled() -> bool {
    matches!(autostart_status(), AutostartStatus::Current)
}

pub(crate) fn repair_autostart_if_stale(
    state_config: &AppStateConfig,
) -> Result<(), Box<dyn Error>> {
    if matches!(autostart_status(), AutostartStatus::Stale) {
        set_autostart_current_exe(state_config.run_as_admin)?;
    }
    Ok(())
}

pub(crate) fn set_autostart(_paths: &AppPaths, highest: bool) -> Result<(), Box<dyn Error>> {
    set_autostart_current_exe(highest)
}

fn set_autostart_current_exe(highest: bool) -> Result<(), Box<dyn Error>> {
    let exe = std::env::current_exe()?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutostartStatus {
    Missing,
    Current,
    Stale,
}

fn autostart_status() -> AutostartStatus {
    let Some(command) = query_autostart_command() else {
        return AutostartStatus::Missing;
    };
    let Ok(current_exe) = std::env::current_exe() else {
        return AutostartStatus::Stale;
    };
    if same_exe_path(&command, &current_exe.to_string_lossy()) {
        AutostartStatus::Current
    } else {
        AutostartStatus::Stale
    }
}

fn query_autostart_command() -> Option<String> {
    let mut command = Command::new("schtasks");
    let output = hide_window(
        command
            .args(["/Query", "/TN", TASK_NAME, "/XML"])
            .stderr(Stdio::null()),
    )
    .output()
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let xml = String::from_utf8_lossy(&output.stdout);
    xml_tag_text(&xml, "Command").map(unescape_xml)
}

fn xml_tag_text(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;
    Some(xml[start..end].to_string())
}

fn unescape_xml(value: String) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn same_exe_path(task_command: &str, current_exe: &str) -> bool {
    normalize_exe_path(task_command).eq_ignore_ascii_case(&normalize_exe_path(current_exe))
}

fn normalize_exe_path(path: &str) -> String {
    path.trim().trim_matches('"').replace('/', "\\")
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
