use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use singboost::{
    AppPaths, SubscriptionConfig, SubscriptionError, download_subscription,
    resolve_subscription_target, write_subscription_content,
};

#[test]
fn rejects_missing_target_instead_of_falling_back() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());

    let error = resolve_subscription_target(&paths, None).unwrap_err();

    assert!(matches!(error, SubscriptionError::MissingTarget));
}

#[test]
fn rejects_empty_absolute_and_parent_targets() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());

    assert!(matches!(
        resolve_subscription_target(&paths, Some("   ")).unwrap_err(),
        SubscriptionError::EmptyTarget
    ));
    assert!(matches!(
        resolve_subscription_target(&paths, Some("/tmp/config.json")).unwrap_err(),
        SubscriptionError::InvalidTarget(_)
    ));
    assert!(matches!(
        resolve_subscription_target(&paths, Some("../config.json")).unwrap_err(),
        SubscriptionError::InvalidTarget(_)
    ));
}

#[test]
fn writes_subscription_content_atomically_and_rejects_empty_content() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.config_json(), "old").unwrap();

    write_subscription_content(&paths.config_json(), br#"{"log":{"level":"info"}}"#).unwrap();
    assert_eq!(
        std::fs::read_to_string(paths.config_json()).unwrap(),
        "{\n  \"log\": {\n    \"level\": \"info\"\n  }\n}"
    );

    let err = write_subscription_content(&paths.config_json(), b"  \n\t").unwrap_err();
    assert!(matches!(err, SubscriptionError::EmptyResponse));
    assert_eq!(
        std::fs::read_to_string(paths.config_json()).unwrap(),
        "{\n  \"log\": {\n    \"level\": \"info\"\n  }\n}"
    );
}

#[test]
fn downloads_subscription_to_configured_target_as_pretty_json() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    let url = spawn_http_server(r#"{"outbounds":[{"type":"direct"}]}"#);

    download_subscription(
        &paths,
        &SubscriptionConfig {
            url: Some(url),
            target: Some("downloaded.json".to_string()),
        },
    )
    .unwrap();

    assert_eq!(
        std::fs::read_to_string(temp.path().join("downloaded.json")).unwrap(),
        "{\n  \"outbounds\": [\n    {\n      \"type\": \"direct\"\n    }\n  ]\n}"
    );
}

#[test]
fn rejects_invalid_json_download_without_overwriting_existing_config() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    std::fs::write(paths.config_json(), "old").unwrap();

    let err = write_subscription_content(&paths.config_json(), b"not json").unwrap_err();

    assert!(matches!(err, SubscriptionError::InvalidJson(_)));
    assert_eq!(std::fs::read_to_string(paths.config_json()).unwrap(), "old");
}

#[test]
fn rejects_empty_download_response() {
    let temp = tempfile::tempdir().unwrap();
    let paths = AppPaths::new(temp.path().to_path_buf());
    let url = spawn_http_server("  \n");

    let err = download_subscription(
        &paths,
        &SubscriptionConfig {
            url: Some(url),
            target: Some("config.json".to_string()),
        },
    )
    .unwrap_err();

    assert!(matches!(err, SubscriptionError::EmptyResponse));
    assert!(!paths.config_json().exists());
}

fn spawn_http_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 1024];
        let _ = stream.read(&mut request).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    format!("http://{addr}/config.json")
}
