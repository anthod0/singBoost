#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
mod windows_app;

#[cfg(windows)]
fn main() {
    if let Err(err) = windows_app::run() {
        windows_app::show_error(&format!("SingBoost failed: {err}"));
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("SingBoost is a Windows-only tray launcher.");
}
