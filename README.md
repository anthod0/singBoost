# SingBoost

A minimal Windows launcher for the sing-box core.

## Features

- Start/stop the sing-box core
- View runtime logs
- Quick open the sing-box Web UI
- Configure startup on login
- Windows system tray icon

## Non-goals

- Does not generate or modify sing-box config files
- Does not provide common GUI interfaces for subscriptions, config, proxy switching, etc.
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

On first launch, if `boost.toml` does not exist, SingBoost creates a default configuration file automatically.

## Configuration File

Configuration file path:

```text
<your_app_dir>\boost.toml
```

Default content:

```toml
[app]
run_as_admin = false

[sing_box]
start_command = 'sing-box.exe -D . -c config.json run'
```

## Tray Menu

Right-click the tray icon to access:

- `Start / Stop / Restart`: manage sing-box core state.
- Open UI: open the Web UI from `config.json`.
- Logs: open a PowerShell window that tails `logs\singboost-runtime.log` in real time.
- Run as administrator: toggle the administrator privilege setting.
- Startup on login: toggle the windows startup task.
- Exit: stop sing-box core, close log windows opened by SingBoost, and exit.

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
