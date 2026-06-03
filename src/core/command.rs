use crate::core::config::AppConfig;
use crate::core::paths::{AppPaths, append_child, looks_like_windows_path};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
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

    pub fn run(paths: &AppPaths, config: &AppConfig) -> Result<Self, CommandLineError> {
        spawn_command_line(&config.start_command, &paths.app_dir())
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

fn path_arg(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}
