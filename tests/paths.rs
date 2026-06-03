use std::path::PathBuf;

use singboost::AppPaths;

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
        paths.config_toml(),
        PathBuf::from(r"D:\Program Files\sing-box\singboost.toml")
    );
    assert_eq!(
        paths.runtime_log(),
        PathBuf::from(r"D:\Program Files\sing-box\logs\singboost-runtime.log")
    );
}
