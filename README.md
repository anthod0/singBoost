# SingBoost

A minimal Windows launcher for the sing-box core.

## Features

- Start/stop the sing-box core
- View runtime logs
- Quick open the sing-box Web UI
- Configure startup on login
- Download a complete remote sing-box JSON config on demand
- Download or update the sing-box core from official stable releases
- Windows system tray icon

## Non-goals

- Does not generate or convert sing-box config files
- Does not provide common GUI interfaces for node subscriptions, config generation, proxy switching, etc.
- Does not bundle the sing-box core
- Does not support non-Windows platforms

## Prerequisites

- A complete sing-box 1.14 config file
- The official sing-box Windows runtime files, or use the tray menu to download them

## Target Directory Layout

Place `singboost.exe` in the target application directory. `sing-box.exe` can be provided manually or downloaded from the tray menu:

```text
<your_app_dir>\
  singboost.exe
  sing-box.exe
  libcronet.dll
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

sing-box supports loading multiple config files with repeated `-c` options. sing-box sorts config paths before merging: earlier files have priority for scalar fields, while array fields are appended.

This is useful for adding local settings to a downloaded remote config.

Example:

```toml
[sing_box]
start_command = 'sing-box.exe -D . -c 00-local.json -c config.json run'

[subscription]
url = ""
target = "config.json"
# Optional, defaults to 30 seconds. Valid range: 1..=300.
timeout_secs = 30
```

In this example, `00-local.json` is loaded before `config.json`, so local scalar fields such as `log.level` take priority over the downloaded remote config. Array fields are appended.

Remote config downloads time out after `subscription.timeout_secs` seconds, or 30 seconds if omitted.

## Core Management

The **Kernel** tray submenu checks the official stable release published by
[SagerNet/sing-box](https://github.com/SagerNet/sing-box/releases). SingBoost selects the
Windows AMD64 or ARM64 archive that matches its own build architecture.

A user-confirmed installation or update:

- runs in the background while release information and the archive are downloaded;
- verifies the archive size and GitHub-provided SHA-256 digest;
- extracts only the expected `sing-box.exe` and `libcronet.dll`, then verifies the executable's reported version;
- stops a running core only after the replacement is ready;
- restores the previous runtime files if replacement fails; and
- restarts the core if it was running before the update.

Only the latest stable release is supported. Pre-releases, automatic updates, and historical
version selection are not provided. Downloading the core does not modify sing-box configuration.

## Web UI

SingBoost supports both Web UI configurations available in sing-box 1.14. If both are enabled, the native sing-box API Dashboard takes priority.

Native API Dashboard:

```json
{
  "services": [
    {
      "type": "api",
      "listen": "127.0.0.1",
      "listen_port": 9090,
      "dashboard": true
    }
  ]
}
```

Clash API UI remains supported:

```json
{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "external_ui": "ui"
    }
  }
}
```

SingBoost opens `/dashboard/` for the native API service and `/ui/` for the Clash API. It does not create or migrate either configuration.

## Tray Menu

Left-click the tray icon to open the Web UI only when the sing-box core is running.

Right-click for common actions:

- Manage sing-box: start, stop, or restart the core.
- Open UI and logs.
- Configuration shortcuts.
- Check, download, or update the sing-box core.
- Toggle administrator mode.
- Toggle startup on login. SingBoost uses a fixed Windows Task Scheduler task name and silently repairs an existing startup task if a portable upgrade moved or renamed the current executable.
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
