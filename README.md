# SingBoost

SingBoost is a minimal Windows launcher for the sing-box core. It provides a system tray icon, log viewing, Web UI opening, startup on login, and optional administrator privileges.

## Features

- Start/stop the sing-box core
- View runtime logs
- Windows system tray icon
- Open the sing-box Web UI
- Configure startup on login
- Optionally run SingBoost, and therefore the sing-box child process, as administrator

## Non-goals

- Does not generate or modify sing-box configuration
- Does not parse proxy nodes
- Does not bundle the sing-box core

## Prerequisites

- A complete sing-box configuration prepared by the user
- `sing-box.exe`

## Target Directory Layout

Place `singboost.exe` in the same directory as `sing-box.exe`:

```text
<app_dir>\
  singboost.exe
  sing-box.exe
  config.json
```

On first launch, if `boost.toml` does not exist, SingBoost creates a default configuration file automatically.

## Configuration File

Configuration file path:

```text
<app_dir>\boost.toml
```

Default content:

```toml
[app]
run_as_admin = false

[sing_box]
start_command = 'sing-box.exe -D . -c config.json run'
```

Notes:

- When `run_as_admin = true`, SingBoost relaunches itself through UAC after startup.
- The sing-box child process inherits the current SingBoost process privileges.
- `start_command` can be customized by the user.
- If the configuration file already exists but its TOML cannot be parsed, required fields are missing, or `start_command` is empty, SingBoost will not start sing-box.

## Tray Menu

Right-click the tray icon to access:

- Start / Starting... / Stop: start sing-box, show the starting state, or stop sing-box. The menu item is disabled while starting to avoid duplicate launches.
- Restart: available only while sing-box is running.
- Open UI: open the Web UI from `config.json` at `experimental.clash_api.external_controller`.
- Logs: open a PowerShell window that tails `logs\singboost-runtime.log` in real time.
- Run as administrator: toggle the administrator privilege setting.
- Startup on login: create or remove the Windows Task Scheduler startup task.
- Exit: stop sing-box, close log windows opened by SingBoost, and exit.

## Logs

SingBoost recreates this file each time it starts:

```text
<app_dir>\logs\singboost-runtime.log
```

Log sources include:

- sing-box stdout
- sing-box stderr
- SingBoost events and errors

Click Logs in the tray menu to view live log output.

## Build

### Native Windows Build

Development build:

```powershell
cargo build
```

Release build:

```powershell
cargo build --release
```

Build artifacts:

```text
target\debug\singboost.exe
target\release\singboost.exe
```

For normal use, prefer `target\release\singboost.exe`.

### Linux/macOS Build Notes

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

Building and verifying tray behavior, Task Scheduler integration, and administrator privilege behavior on Windows is recommended.
