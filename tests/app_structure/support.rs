pub(crate) fn read_tray_app_sources() -> String {
    [
        "src/windows_app/tray_app.rs",
        "src/windows_app/tray_app/kernel.rs",
        "src/windows_app/tray_app/menu_actions.rs",
        "src/windows_app/tray_app/ui_state.rs",
    ]
    .into_iter()
    .map(|path| std::fs::read_to_string(path).unwrap().replace("\r\n", "\n"))
    .collect::<Vec<_>>()
    .join("\n")
}
