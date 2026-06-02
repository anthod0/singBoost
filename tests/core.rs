use std::path::PathBuf;

use singboost::{AppPaths, KernelCommand};

#[test]
fn app_paths_are_derived_from_app_dir() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));

    assert_eq!(paths.app_dir(), PathBuf::from(r"D:\Program Files\sing-box"));
    assert_eq!(
        paths.sing_box_exe(),
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        paths.config_json(),
        PathBuf::from(r"D:\Program Files\sing-box\config.json")
    );
    assert_eq!(
        paths.logs_dir(),
        PathBuf::from(r"D:\Program Files\sing-box\logs")
    );
    assert_eq!(
        paths.stdout_log(),
        PathBuf::from(r"D:\Program Files\sing-box\logs\sing-box.stdout.log")
    );
    assert_eq!(
        paths.stderr_log(),
        PathBuf::from(r"D:\Program Files\sing-box\logs\sing-box.stderr.log")
    );
    assert_eq!(
        paths.singboost_log(),
        PathBuf::from(r"D:\Program Files\sing-box\logs\singboost.log")
    );
}

#[test]
fn sing_box_check_command_matches_spec() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let command = KernelCommand::check(&paths);

    assert_eq!(
        command.program,
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        command.args,
        vec![
            "check".to_string(),
            "-D".to_string(),
            r"D:\Program Files\sing-box".to_string(),
            "-c".to_string(),
            r"D:\Program Files\sing-box\config.json".to_string(),
        ]
    );
}

#[test]
fn sing_box_run_command_matches_spec() {
    let paths = AppPaths::new(PathBuf::from(r"D:\Program Files\sing-box"));
    let command = KernelCommand::run(&paths);

    assert_eq!(
        command.program,
        PathBuf::from(r"D:\Program Files\sing-box\sing-box.exe")
    );
    assert_eq!(
        command.args,
        vec![
            "-D".to_string(),
            r"D:\Program Files\sing-box".to_string(),
            "-c".to_string(),
            r"D:\Program Files\sing-box\config.json".to_string(),
            "run".to_string(),
        ]
    );
}
