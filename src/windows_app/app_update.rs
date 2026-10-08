use singboost::core::app_update::{
    PreparedAppUpdate, STAGING_PREFIX, cleanup_update_directory, install_app_update,
};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    QueryFullProcessImageNameW, WaitForSingleObject,
};
use windows::core::PWSTR;

type UpdateResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
fn publish_marker(directory: &Path, name: &str) -> std::io::Result<()> {
    let temporary = directory.join(format!("{name}.tmp"));
    std::fs::write(&temporary, name.as_bytes())?;
    std::fs::rename(temporary, directory.join(name))
}
struct Process(HANDLE);
// Owned kernel process handles may be waited on and closed from any thread.
unsafe impl Send for Process {}
impl Process {
    fn wait(&self) -> bool {
        unsafe { WaitForSingleObject(self.0, u32::MAX) == WAIT_OBJECT_0 }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}
fn open_process(pid: u32) -> UpdateResult<Process> {
    Ok(Process(unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            false,
            pid,
        )
    }?))
}
fn process_path(process: &Process) -> UpdateResult<PathBuf> {
    let mut buffer = vec![0u16; 32768];
    let mut length = buffer.len() as u32;
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(buffer.as_mut_ptr()),
            &mut length,
        )
    }?;
    Ok(PathBuf::from(String::from_utf16(&buffer[..length as usize])?).canonicalize()?)
}
fn validate_directory(directory: &Path, target: &Path) -> UpdateResult<PathBuf> {
    let directory = directory.canonicalize()?;
    if directory.parent() != target.parent()
        || !directory
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with(STAGING_PREFIX))
    {
        return Err("invalid update staging directory".into());
    }
    Ok(directory)
}

/// Called off the UI thread. The helper must hold the parent handle before it may exit.
pub(crate) fn start_helper(prepared: &mut PreparedAppUpdate) -> UpdateResult<()> {
    let target = std::env::current_exe()?.canonicalize()?;
    let mut helper = Command::new(prepared.directory.join("helper.exe"))
        .arg("--apply-app-update")
        .arg(std::process::id().to_string())
        .arg(&target)
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if helper.try_wait()?.is_some() {
            return Err("update helper exited before becoming ready".into());
        }
        if prepared.directory.join("ready").is_file() {
            prepared.hand_off();
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = helper.kill();
    let _ = helper.wait();
    Err("update helper startup timed out".into())
}

pub(crate) enum StartupMode {
    Normal,
    HelperFinished,
    Updated(UpdateStartup),
}
pub(crate) struct UpdateStartup {
    directory: PathBuf,
    helper: Process,
}
impl UpdateStartup {
    /// Acknowledge only after configuration, single-instance lock and tray initialization succeeded.
    pub(crate) fn acknowledge(self) -> Result<(), Box<dyn Error>> {
        publish_marker(&self.directory, "started")?;
        std::thread::spawn(move || {
            if self.helper.wait() {
                cleanup_update_directory(&self.directory);
            }
        });
        Ok(())
    }
}

// Internal modes are handled before configuration, elevation and single-instance acquisition.
pub(crate) fn handle_internal_mode() -> Result<StartupMode, Box<dyn Error>> {
    handle_internal_mode_impl().map_err(|e| -> Box<dyn Error> { e })
}
fn handle_internal_mode_impl() -> UpdateResult<StartupMode> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(mode) = args.first().and_then(|a| a.to_str()) else {
        return Ok(StartupMode::Normal);
    };
    if mode != "--apply-app-update" && mode != "--finish-app-update" {
        return Ok(StartupMode::Normal);
    }
    if args.len() != 3 {
        return Err("invalid update arguments".into());
    }
    let pid: u32 = args[1].to_str().ok_or("invalid process ID")?.parse()?;
    if pid == std::process::id() {
        return Err("invalid update process ID".into());
    }
    if mode == "--finish-app-update" {
        let target = std::env::current_exe()?.canonicalize()?;
        let directory = validate_directory(Path::new(&args[2]), &target)?;
        let helper = open_process(pid)?;
        if process_path(&helper)? != directory.join("helper.exe") {
            return Err("cleanup process is not the update helper".into());
        }
        return Ok(StartupMode::Updated(UpdateStartup { directory, helper }));
    }
    let target = PathBuf::from(&args[2]).canonicalize()?;
    let helper_path = std::env::current_exe()?.canonicalize()?;
    let directory = validate_directory(
        helper_path.parent().ok_or("missing staging directory")?,
        &target,
    )?;
    if helper_path != directory.join("helper.exe") {
        return Err("invalid update helper path".into());
    }
    let parent = open_process(pid)?;
    if process_path(&parent)? != target {
        return Err("update parent does not match target".into());
    }
    singboost::core::app_update::validate_executable(&std::fs::read(directory.join("new.exe"))?)?;
    publish_marker(&directory, "ready")?;
    // Modal dialogs can delay delivery of the UI event arbitrarily. Never expire readiness.
    if !parent.wait() {
        return Err("failed to wait for application exit; no files were replaced".into());
    }
    install_app_update(&target, &directory, |exe| {
        let started = directory.join("started");
        if started.exists() {
            std::fs::remove_file(&started)?;
        }
        let mut child = Command::new(exe)
            .arg("--finish-app-update")
            .arg(std::process::id().to_string())
            .arg(&directory)
            .current_dir(target.parent().ok_or("missing application directory")?)
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(60);
        while Instant::now() < deadline {
            if started.is_file() {
                return Ok(());
            }
            match child.try_wait() {
                Ok(Some(_)) => {
                    if started.is_file() {
                        return Ok(());
                    }
                    return Err("updated application exited during startup".into());
                }
                Ok(None) => {}
                Err(error) => {
                    child.kill()?;
                    child.wait()?;
                    return Err(error.into());
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if started.is_file() {
            return Ok(());
        }
        // Restoration must not run while the unsuccessful replacement is still alive.
        child.kill()?;
        child.wait()?;
        Err("updated application did not acknowledge startup".into())
    })?;
    Ok(StartupMode::HelperFinished)
}
