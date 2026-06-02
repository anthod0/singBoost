#[cfg(windows)]
mod windows_app;

#[cfg(windows)]
fn main() {
    if let Err(err) = windows_app::run() {
        eprintln!("SingBoost failed: {err}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("SingBoost is a Windows-only tray launcher.");
}
