use serde::Deserialize;
use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

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
        self.child("singboost.toml")
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub run_as_admin: bool,
    pub start_command: String,
}

impl AppConfig {
    pub fn default_for_app_dir(_app_dir: &Path) -> Self {
        Self {
            run_as_admin: false,
            start_command: r#"sing-box.exe -D "<app_dir>" -c "<app_dir>\config.json" run"#
                .to_string(),
        }
    }

    pub fn default_toml() -> &'static str {
        concat!(
            "[app]\n",
            "run_as_admin = false\n\n",
            "[sing_box]\n",
            "start_command = 'sing-box.exe -D \"<app_dir>\" -c \"<app_dir>\\config.json\" run'\n",
        )
    }

    pub fn expanded_start_command(&self, app_dir: &Path) -> String {
        self.start_command
            .replace("<app_dir>", &app_dir.to_string_lossy())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] io::Error),
    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing app.run_as_admin")]
    MissingRunAsAdmin,
    #[error("missing sing_box.start_command")]
    MissingStartCommand,
    #[error("sing_box.start_command must not be empty")]
    EmptyStartCommand,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    app: Option<RawAppConfig>,
    sing_box: Option<RawSingBoxConfig>,
}

#[derive(Debug, Deserialize)]
struct RawAppConfig {
    run_as_admin: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawSingBoxConfig {
    start_command: Option<String>,
}

pub fn ensure_config_file(paths: &AppPaths) -> io::Result<()> {
    let config_path = paths.config_toml();
    if !config_path.exists() {
        std::fs::write(config_path, AppConfig::default_toml())?;
    }
    Ok(())
}

pub fn load_config(paths: &AppPaths) -> Result<AppConfig, ConfigError> {
    let text = std::fs::read_to_string(paths.config_toml())?;
    let raw: RawConfig = toml::from_str(&text)?;
    let run_as_admin = raw
        .app
        .and_then(|app| app.run_as_admin)
        .ok_or(ConfigError::MissingRunAsAdmin)?;
    let start_command = raw
        .sing_box
        .and_then(|sing_box| sing_box.start_command)
        .ok_or(ConfigError::MissingStartCommand)?;
    if start_command.trim().is_empty() {
        return Err(ConfigError::EmptyStartCommand);
    }
    Ok(AppConfig {
        run_as_admin,
        start_command,
    })
}

#[derive(Debug, Error)]
pub enum CommandLineError {
    #[error("command line is empty")]
    Empty,
    #[error("unterminated quoted string")]
    UnterminatedQuote,
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

    pub fn run(paths: &AppPaths, config: &AppConfig) -> Self {
        let expanded = config.expanded_start_command(&paths.app_dir());
        spawn_command_line(&expanded, &paths.app_dir()).expect("validated start command")
    }
}

pub fn spawn_command_line(
    command_line: &str,
    app_dir: &Path,
) -> Result<KernelCommand, CommandLineError> {
    let mut parts = split_windows_command_line(command_line)?;
    if parts.is_empty() {
        return Err(CommandLineError::Empty);
    }
    let program = parts.remove(0);
    let program_path =
        if Path::new(&program).is_absolute() || looks_like_windows_path(Path::new(&program)) {
            PathBuf::from(program)
        } else {
            append_child(app_dir, &program)
        };
    Ok(KernelCommand {
        program: program_path,
        args: parts,
    })
}

fn split_windows_command_line(input: &str) -> Result<Vec<String>, CommandLineError> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut saw_any = false;

    for ch in input.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                saw_any = true;
            }
            c if c.is_whitespace() && !in_quotes => {
                if saw_any {
                    args.push(std::mem::take(&mut current));
                    saw_any = false;
                }
            }
            c => {
                current.push(c);
                saw_any = true;
            }
        }
    }

    if in_quotes {
        return Err(CommandLineError::UnterminatedQuote);
    }
    if saw_any {
        args.push(current);
    }
    Ok(args)
}

#[derive(Debug, Error)]
pub enum PreflightError {
    #[error("missing sing-box.exe: {0}")]
    MissingSingBox(PathBuf),
    #[error("missing config.json: {0}")]
    MissingConfig(PathBuf),
    #[error("failed to create logs directory: {0}")]
    LogsDir(io::Error),
    #[error("failed to recreate runtime log: {0}")]
    RuntimeLog(io::Error),
}

pub fn validate_preflight_files(paths: &AppPaths) -> Result<(), PreflightError> {
    if !paths.sing_box_exe().exists() {
        return Err(PreflightError::MissingSingBox(paths.sing_box_exe()));
    }
    if !paths.config_json().exists() {
        return Err(PreflightError::MissingConfig(paths.config_json()));
    }
    std::fs::create_dir_all(paths.logs_dir()).map_err(PreflightError::LogsDir)?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(paths.runtime_log())
        .map_err(PreflightError::RuntimeLog)?;
    Ok(())
}

pub struct RuntimeLog {
    file: File,
}

impl RuntimeLog {
    pub fn recreate(paths: &AppPaths) -> io::Result<Self> {
        std::fs::create_dir_all(paths.logs_dir())?;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(paths.runtime_log())?;
        Ok(Self { file })
    }

    pub fn append_event(&mut self, message: impl AsRef<str>) -> io::Result<()> {
        writeln!(self.file, "[{}] {}", timestamp(), message.as_ref())?;
        self.file.flush()
    }
}

fn timestamp() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
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
