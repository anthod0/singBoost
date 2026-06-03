use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    app_dir: PathBuf,
}

impl AppPaths {
    pub fn new(app_dir: PathBuf) -> Self {
        Self { app_dir }
    }

    pub fn from_current_exe() -> io::Result<Self> {
        let exe = std::env::current_exe()?;
        let app_dir = exe
            .parent()
            .ok_or_else(|| io::Error::other("current executable has no parent directory"))?
            .to_path_buf();
        Ok(Self::new(app_dir))
    }

    pub fn app_dir(&self) -> PathBuf {
        self.app_dir.clone()
    }

    pub fn config_toml(&self) -> PathBuf {
        self.child("boost.toml")
    }

    pub fn state_toml(&self) -> PathBuf {
        self.child("boost.state.toml")
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

    pub fn runtime_log(&self) -> PathBuf {
        self.logs_child("singboost-runtime.log")
    }

    fn logs_child(&self, file_name: &str) -> PathBuf {
        append_child(&self.logs_dir(), file_name)
    }

    fn child(&self, file_name: &str) -> PathBuf {
        append_child(&self.app_dir, file_name)
    }
}

pub(crate) fn append_child(parent: &Path, child: &str) -> PathBuf {
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

pub(crate) fn looks_like_windows_path(path: &Path) -> bool {
    let s = path.as_os_str().to_string_lossy();
    s.contains('\\')
        || s.as_bytes().get(1) == Some(&b':')
        || path.components().any(|c| c.as_os_str() == OsStr::new(r"\"))
}
