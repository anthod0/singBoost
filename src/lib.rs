use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppState {
    Stopped,
    Running,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    app_dir: PathBuf,
}

impl AppPaths {
    pub fn new(app_dir: PathBuf) -> Self {
        Self { app_dir }
    }

    pub fn from_current_exe() -> std::io::Result<Self> {
        let exe = std::env::current_exe()?;
        let app_dir = exe
            .parent()
            .ok_or_else(|| std::io::Error::other("current executable has no parent directory"))?
            .to_path_buf();
        Ok(Self::new(app_dir))
    }

    pub fn app_dir(&self) -> PathBuf {
        self.app_dir.clone()
    }

    pub fn sing_box_exe(&self) -> PathBuf {
        self.child("sing-box.exe")
    }

    pub fn config_json(&self) -> PathBuf {
        self.child("config.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.child("logs")
    }

    pub fn stdout_log(&self) -> PathBuf {
        self.logs_child("sing-box.stdout.log")
    }

    pub fn stderr_log(&self) -> PathBuf {
        self.logs_child("sing-box.stderr.log")
    }

    pub fn singboost_log(&self) -> PathBuf {
        self.logs_child("singboost.log")
    }

    fn logs_child(&self, file_name: &str) -> PathBuf {
        append_child(&self.logs_dir(), file_name)
    }

    fn child(&self, file_name: &str) -> PathBuf {
        append_child(&self.app_dir, file_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
}

impl KernelCommand {
    pub fn check(paths: &AppPaths) -> Self {
        Self {
            program: paths.sing_box_exe(),
            args: vec![
                "check".to_string(),
                "-D".to_string(),
                path_arg(paths.app_dir()),
                "-c".to_string(),
                path_arg(paths.config_json()),
            ],
        }
    }

    pub fn run(paths: &AppPaths) -> Self {
        Self {
            program: paths.sing_box_exe(),
            args: vec![
                "-D".to_string(),
                path_arg(paths.app_dir()),
                "-c".to_string(),
                path_arg(paths.config_json()),
                "run".to_string(),
            ],
        }
    }
}

fn path_arg(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn append_child(parent: &Path, child: &str) -> PathBuf {
    if looks_like_windows_path(parent) {
        let mut base = parent.to_string_lossy().into_owned();
        while base.ends_with('\\') || base.ends_with('/') {
            base.pop();
        }
        PathBuf::from(format!(r"{}\{}", base, child))
    } else {
        parent.join(child)
    }
}

fn looks_like_windows_path(path: &Path) -> bool {
    let s = path.as_os_str().to_string_lossy();
    s.contains('\\')
        || s.as_bytes().get(1) == Some(&b':')
        || path.components().any(|c| c.as_os_str() == OsStr::new(r"\"))
}
