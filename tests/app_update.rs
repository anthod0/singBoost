use sha2::{Digest, Sha256};
use singboost::core::app_update::{AppUpdate, install_app_update};
use std::cell::Cell;

fn executable() -> Vec<u8> {
    let mut bytes = vec![0u8; 512];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
    bytes[128..132].copy_from_slice(b"PE\0\0");
    bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    bytes[150..152].copy_from_slice(&0x0002u16.to_le_bytes());
    bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
    bytes
}
fn release(version: &str, bytes: &[u8]) -> serde_json::Value {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    serde_json::json!({
        "tag_name": format!("v{version}"), "draft": false, "prerelease": false,
        "assets": [{
            "name": "singboost-windows-x86_64.exe",
            "browser_download_url": format!("https://github.com/anthod0/singBoost/releases/download/v{version}/singboost-windows-x86_64.exe"),
            "size": bytes.len(), "digest": format!("sha256:{digest}")
        }]
    })
}
fn parse(value: serde_json::Value) -> AppUpdate {
    AppUpdate::from_release_json(&serde_json::to_vec(&value).unwrap()).unwrap()
}

#[test]
fn only_newer_stable_versions_offer_an_update() {
    let bytes = executable();
    assert!(!parse(release(env!("CARGO_PKG_VERSION"), &bytes)).available());
    assert!(!parse(release("0.0.1", &bytes)).available());
    assert!(parse(release("999.0.0", &bytes)).available());
    let mut prerelease = release("999.0.0", &bytes);
    prerelease["prerelease"] = true.into();
    assert!(AppUpdate::from_release_json(&serde_json::to_vec(&prerelease).unwrap()).is_err());
}

#[test]
fn untrusted_or_unverifiable_release_assets_are_rejected() {
    for field in ["digest", "browser_download_url", "name"] {
        let mut metadata = release("999.0.0", &executable());
        metadata["assets"][0][field] = "invalid".into();
        assert!(AppUpdate::from_release_json(&serde_json::to_vec(&metadata).unwrap()).is_err());
    }
}

#[test]
fn modified_or_truncated_downloads_are_rejected() {
    let mut bytes = executable();
    let update = parse(release("999.0.0", &bytes));
    assert!(update.validate_download(&bytes).is_ok());
    bytes[511] = 1;
    assert!(update.validate_download(&bytes).is_err());
    bytes.pop();
    assert!(update.validate_download(&bytes).is_err());
}

#[test]
fn matching_digest_does_not_make_invalid_executable_safe() {
    let mut bytes = executable();
    bytes[132..134].copy_from_slice(&0x014cu16.to_le_bytes());
    assert!(
        parse(release("999.0.0", &bytes))
            .validate_download(&bytes)
            .is_err()
    );
    let bytes = b"not an executable";
    assert!(
        parse(release("999.0.0", bytes))
            .validate_download(bytes)
            .is_err()
    );
}

#[test]
fn successful_install_replaces_only_application_executable() {
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("staging");
    std::fs::create_dir(&staging).unwrap();
    let target = root.path().join("renamed-singboost.exe");
    std::fs::write(&target, b"old executable").unwrap();
    for name in [
        "boost.toml",
        "boost.state.toml",
        "config.json",
        "sing-box.exe",
    ] {
        std::fs::write(root.path().join(name), name.as_bytes()).unwrap();
    }
    let new = executable();
    std::fs::write(staging.join("new.exe"), &new).unwrap();
    install_app_update(&target, &staging, |path| {
        assert_eq!(path, target);
        assert_eq!(std::fs::read(path).unwrap(), new);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        std::fs::read(staging.join("old.exe")).unwrap(),
        b"old executable"
    );
    for name in [
        "boost.toml",
        "boost.state.toml",
        "config.json",
        "sing-box.exe",
    ] {
        assert_eq!(
            std::fs::read(root.path().join(name)).unwrap(),
            name.as_bytes()
        );
    }
}

#[test]
fn failed_start_restores_and_restarts_old_application() {
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("staging");
    std::fs::create_dir(&staging).unwrap();
    let target = root.path().join("singboost.exe");
    std::fs::write(&target, b"original").unwrap();
    std::fs::write(staging.join("new.exe"), executable()).unwrap();
    let attempts = Cell::new(0);
    let result = install_app_update(&target, &staging, |path| {
        attempts.set(attempts.get() + 1);
        if attempts.get() == 1 {
            return Err("startup failed".into());
        }
        assert_eq!(std::fs::read(path).unwrap(), b"original");
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(attempts.get(), 2);
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
}

#[test]
fn invalid_replacement_leaves_original_untouched() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("singboost.exe");
    std::fs::write(&target, b"original").unwrap();
    std::fs::write(root.path().join("new.exe"), b"bad download").unwrap();
    assert!(install_app_update(&target, root.path(), |_| panic!("must not launch")).is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"original");
}
