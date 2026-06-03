use singboost::{AppPaths, RuntimeLog};

#[test]
fn runtime_log_path_is_fixed_and_recreated() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::create_dir_all(paths.logs_dir()).unwrap();
    std::fs::write(paths.runtime_log(), "old contents").unwrap();

    let mut log = RuntimeLog::recreate(&paths).unwrap();
    log.append_event("started").unwrap();

    let content = std::fs::read_to_string(paths.runtime_log()).unwrap();
    assert!(!content.contains("old contents"));
    assert!(content.contains("started"));
}
