#[test]
fn source_is_split_by_responsibility() {
    for path in [
        "src/core/paths.rs",
        "src/core/config.rs",
        "src/core/command.rs",
        "src/core/preflight.rs",
        "src/core/runtime_log.rs",
        "src/core/web_ui.rs",
        "src/windows_app/tray_app.rs",
        "src/windows_app/tray_menu.rs",
        "src/windows_app/process.rs",
        "src/windows_app/autostart.rs",
        "src/windows_app/elevation.rs",
        "src/windows_app/error_dialog.rs",
        "src/windows_app/single_instance.rs",
    ] {
        assert!(std::path::Path::new(path).exists(), "missing {path}");
    }
}

#[test]
fn tray_icon_asset_is_a_32px_rgba_image() {
    let icon = std::fs::read("assets/tray-icon.rgba").unwrap();

    assert_eq!(icon.len(), 32 * 32 * 4);
    assert!(icon.chunks_exact(4).any(|pixel| pixel[3] > 0));
}
