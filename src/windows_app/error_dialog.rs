use windows::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO, MESSAGEBOX_STYLE,
    MessageBoxW,
};
use windows::core::{HSTRING, PCWSTR};

pub(crate) fn show_error(message: &str) {
    show_message("SingBoost", message, MB_OK | MB_ICONERROR);
}

pub(crate) fn show_info(title: &str, message: &str) {
    show_message(title, message, MB_OK | MB_ICONINFORMATION);
}

pub(crate) fn confirm(title: &str, message: &str) -> bool {
    let title = HSTRING::from(title);
    let text = HSTRING::from(message);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONWARNING,
        ) == IDYES
    }
}

fn show_message(title: &str, message: &str, style: MESSAGEBOX_STYLE) {
    let title = HSTRING::from(title);
    let text = HSTRING::from(message);
    unsafe {
        let _ = MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), style);
    }
}
