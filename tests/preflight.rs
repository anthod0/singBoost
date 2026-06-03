use singboost::{AppPaths, PreflightError, validate_preflight_files};

#[test]
fn preflight_reports_missing_sing_box_before_starting() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.config_json(), "{}").unwrap();

    let error = validate_preflight_files(&paths).unwrap_err();

    assert!(matches!(error, PreflightError::MissingSingBox(_)));
}

#[test]
fn preflight_recreates_runtime_log_when_required_files_exist() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.sing_box_exe(), "fake exe").unwrap();
    std::fs::write(paths.config_json(), "{}").unwrap();

    validate_preflight_files(&paths).unwrap();

    assert!(paths.logs_dir().exists());
    assert_eq!(std::fs::read_to_string(paths.runtime_log()).unwrap(), "");
}
