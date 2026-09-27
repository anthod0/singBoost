use singboost::{AppPaths, resolve_web_ui_url};

#[test]
fn prefers_sing_box_1_14_api_dashboard() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "services": [{
                "type": "api",
                "listen": "0.0.0.0",
                "listen_port": 9090,
                "dashboard": true
            }],
            "experimental": {
                "clash_api": {
                    "external_controller": "127.0.0.1:20123"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://127.0.0.1:9090/dashboard/");
}

#[test]
fn resolves_tls_api_dashboard_with_ipv6_listener() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "services": [{
                "type": "api",
                "listen": "2001:db8::1",
                "listen_port": 9443,
                "dashboard": { "enabled": true, "path": "dashboard" },
                "tls": { "enabled": true }
            }]
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "https://[2001:db8::1]:9443/dashboard/");
}

#[test]
fn falls_back_to_clash_ui_when_api_dashboard_is_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(
        paths.config_json(),
        r#"{
            "services": [{
                "type": "api",
                "listen": "127.0.0.1",
                "listen_port": 9090,
                "dashboard": false
            }],
            "experimental": {
                "clash_api": {
                    "external_controller": "127.0.0.1:20123"
                }
            }
        }"#,
    )
    .unwrap();

    let url = resolve_web_ui_url(&paths).unwrap();

    assert_eq!(url, "http://127.0.0.1:20123/ui/");
}

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
