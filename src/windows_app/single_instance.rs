use std::error::Error;
use std::fmt::{Display, Formatter};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WIN32_ERROR,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::{HSTRING, PCWSTR};

const SINGLE_INSTANCE_MUTEX_NAME: &str = "Local\\SingBoost.SingleInstance";

pub(crate) struct SingleInstanceGuard {
    handle: HANDLE,
}

#[derive(Debug)]
pub(crate) enum SingleInstanceError {
    AlreadyRunning,
    Windows(windows::core::Error),
}

impl Display for SingleInstanceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => formatter.write_str("SingBoost is already running."),
            Self::Windows(err) => {
                write!(formatter, "failed to create single-instance guard: {err}")
            }
        }
    }
}

impl Error for SingleInstanceError {}

pub(crate) fn acquire() -> Result<SingleInstanceGuard, SingleInstanceError> {
    let mutex_name = HSTRING::from(SINGLE_INSTANCE_MUTEX_NAME);
    let handle =
        unsafe { CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr())) }.map_err(|err| {
            match WIN32_ERROR::from_error(&err) {
                Some(ERROR_ACCESS_DENIED) => SingleInstanceError::AlreadyRunning,
                _ => SingleInstanceError::Windows(err),
            }
        })?;

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Err(SingleInstanceError::AlreadyRunning);
    }

    Ok(SingleInstanceGuard { handle })
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle) };
    }
}
