use singboost::{AppPaths, resolve_web_ui_url};

#[test]
fn resolves_web_ui_url_from_sing_box_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "experimental": {
                "clash_api": {
                    "external_controller": "0.0.0.0:20123",
                    "external_ui": "ui"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://127.0.0.1:20123/ui/");
}

#[test]
fn resolves_web_ui_url_from_non_loopback_controller() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "experimental": {
                "clash_api": {
                    "external_controller": "192.168.1.10:20123"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://192.168.1.10:20123/ui/");
}

#[test]
fn resolves_web_ui_url_from_port_only_controller() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "experimental": {
                "clash_api": {
                    "external_controller": ":20123"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://127.0.0.1:20123/ui/");
}
