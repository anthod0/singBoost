# SingBoost

A minimal Windows launcher for the sing-box core.

## Features

- Start/stop the sing-box core
- View runtime logs
- Quick open the sing-box Web UI
- Configure startup on login
- Download a complete remote sing-box JSON config on demand
- Windows system tray icon

## Non-goals

- Does not generate or convert sing-box config files
- Does not provide common GUI interfaces for node subscriptions, config generation, proxy switching, etc.
- Does not bundle the sing-box core
- Does not support non-Windows platforms

## Prerequisites

- A complete sing-box config file
- `sing-box.exe`

## Target Directory Layout

Place `singboost.exe` in the same directory as `sing-box.exe`:

```text
<your_app_dir>\
  singboost.exe
  sing-box.exe
  config.json
```

On first launch, SingBoost creates `boost.toml` and `boost.state.toml` automatically if they do not exist.

## Configuration File

Configuration file path:

```text
<your_app_dir>\boost.toml
```

Default content:

```toml
[sing_box]
start_command = 'sing-box.exe -D . -c config.json run'
```

To enable remote config download, uncomment and fill the `[subscription]` example in `boost.toml`.

### Sing-box Config Merging

sing-box supports loading multiple config files with repeated `-c` options. Config files are merged in order by sing-box, and array fields are appended.

This is useful for adding local settings to a downloaded remote config.

Example:

```toml
[sing_box]
start_command = 'sing-box.exe -D . -c config.json -c local.json run'

[subscription]
url = ""
target = "config.json"
```

In this example, `config.json` can be the downloaded remote config, and `local.json` can contain user-maintained local additions.

## Tray Menu

Left-click the tray icon to open the Web UI only when the sing-box core is running.

Right-click for common actions:

- Manage sing-box: start, stop, or restart the core.
- Open UI and logs.
- Configuration shortcuts.
- Toggle administrator mode.
- Toggle startup on login.
- Show About information.
- Exit SingBoost and stop sing-box.

## Logs

SingBoost recreates this file each time it starts:

```text
<your_app_dir>\logs\singboost-runtime.log
```

Log sources include:

- sing-box stdout and stderr
- SingBoost events and errors

Click Logs in the tray menu to view live log output.

## Build

### Native Windows Build

```powershell
cargo build --release
```

Build artifacts:

```text
target\release\singboost.exe
```

### Linux Build Notes

The tray application is supported only on Windows. Building directly on non-Windows platforms only produces a placeholder program, not a usable Windows tray launcher.

To cross-compile a Windows executable on Linux, install the Windows target and build:

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Artifact path:

```text
target/x86_64-pc-windows-gnu/release/singboost.exe
```

## License

MIT
