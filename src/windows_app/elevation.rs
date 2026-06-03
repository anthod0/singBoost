use singboost::AppPaths;
use std::error::Error;
use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR};

pub(crate) fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

pub(crate) fn relaunch_elevated(paths: &AppPaths) -> Result<(), Box<dyn Error>> {
    let exe = HSTRING::from(
        paths
            .app_dir()
            .join("singboost.exe")
            .to_string_lossy()
            .as_ref(),
    );
    let verb = HSTRING::from("runas");
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("ShellExecuteW runas failed".into())
    } else {
        Ok(())
    }
}
