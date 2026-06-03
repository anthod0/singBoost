use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    ES_AUTOHSCROLL, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, GetWindowTextLengthW,
    GetWindowTextW, HMENU, MSG, RegisterClassW, SW_SHOW, SetWindowLongPtrW, SetWindowTextW,
    ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP,
    WS_VISIBLE,
};
use windows::core::{HSTRING, w};

const EDIT_ID: isize = 1001;
const SAVE_ID: isize = 1002;
const CANCEL_ID: isize = 1003;

struct DialogState {
    edit: Cell<HWND>,
    done: Cell<bool>,
    result: RefCell<Option<String>>,
}

pub(crate) fn show_subscription_dialog(initial_url: &str) -> Option<String> {
    unsafe {
        let class_name = w!("SingBoostSubscriptionDialog");
        let instance = GetModuleHandleW(None).ok()?;
        let hinstance = instance.into();
        let wnd_class = WNDCLASSW {
            lpfnWndProc: Some(dialog_proc),
            hInstance: hinstance,
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wnd_class);

        let state = DialogState {
            edit: Cell::new(HWND::default()),
            done: Cell::new(false),
            result: RefCell::new(None),
        };
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!("远程配置"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            420,
            260,
            520,
            150,
            None,
            None,
            Some(hinstance),
            Some((&state as *const DialogState).cast::<c_void>()),
        )
        .ok()?;

        let _ = SetWindowTextW(state.edit.get(), &HSTRING::from(initial_url));
        let _ = ShowWindow(hwnd, SW_SHOW);
        SetFocus(Some(state.edit.get())).ok();

        let mut msg = MSG::default();
        while !state.done.get() && GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        state.result.borrow().clone()
    }
}

unsafe extern "system" fn dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create = lparam.0 as *const CREATESTRUCTW;
            let state = unsafe { (*create).lpCreateParams as *const DialogState };
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };
            unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("STATIC"),
                    w!("订阅地址:"),
                    WS_CHILD | WS_VISIBLE,
                    16,
                    18,
                    80,
                    24,
                    Some(hwnd),
                    None,
                    None,
                    None,
                )
                .ok();
                let edit = CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("EDIT"),
                    w!(""),
                    WS_CHILD
                        | WS_VISIBLE
                        | WS_BORDER
                        | WS_TABSTOP
                        | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                    96,
                    16,
                    390,
                    24,
                    Some(hwnd),
                    Some(HMENU(EDIT_ID as *mut c_void)),
                    None,
                    None,
                )
                .unwrap_or_default();
                (*state).edit.set(edit);
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("BUTTON"),
                    w!("保存并下载"),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                    286,
                    62,
                    100,
                    30,
                    Some(hwnd),
                    Some(HMENU(SAVE_ID as *mut c_void)),
                    None,
                    None,
                )
                .ok();
                CreateWindowExW(
                    WINDOW_EX_STYLE(0),
                    w!("BUTTON"),
                    w!("取消"),
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                    396,
                    62,
                    90,
                    30,
                    Some(hwnd),
                    Some(HMENU(CANCEL_ID as *mut c_void)),
                    None,
                    None,
                )
                .ok();
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xffff) as isize;
            if id == SAVE_ID || id == CANCEL_ID {
                if let Some(state) = unsafe { dialog_state(hwnd) } {
                    if id == SAVE_ID {
                        state
                            .result
                            .replace(Some(unsafe { read_window_text(state.edit.get()) }));
                    }
                    state.done.set(true);
                }
                unsafe { DestroyWindow(hwnd).ok() };
                return LRESULT(0);
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_CLOSE => {
            if let Some(state) = unsafe { dialog_state(hwnd) } {
                state.done.set(true);
            }
            unsafe { DestroyWindow(hwnd).ok() };
            LRESULT(0)
        }
        WM_DESTROY => {
            if let Some(state) = unsafe { dialog_state(hwnd) } {
                state.done.set(true);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn dialog_state(hwnd: HWND) -> Option<&'static DialogState> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const DialogState;
    unsafe { ptr.as_ref() }
}

unsafe fn read_window_text(hwnd: HWND) -> String {
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buffer = vec![0u16; len as usize + 1];
    unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..len as usize])
}
