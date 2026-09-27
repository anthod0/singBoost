# SingBoost

A minimal Windows launcher for the sing-box core.

## Features

- Start or stop the sing-box core
- Quickly open the sing-box Web UI
- View runtime logs
- Configure startup on login
- Download a complete remote sing-box JSON config on demand
- Download or update the sing-box core from official stable releases
- Windows system tray icon

## Non-goals

- Does not generate or convert sing-box config files
- Does not provide additional GUI functionality

## Target Directory Layout

Place `singboost.exe` in the target application directory. `sing-box.exe` can be provided manually or downloaded from the tray menu:

```text
<your_app_dir>\
  singboost.exe
  sing-box.exe
  libcronet.dll
  config.json
```

On first launch, SingBoost automatically creates `boost.toml` and `boost.state.toml` if they do not exist.

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

To enable remote config downloads, uncomment and fill in the `[subscription]` example in `boost.toml`.

### Sing-box Config Merging

sing-box supports loading multiple config files by repeating the `-c` option. This is useful for adding local settings to a downloaded remote config.

Example:

```toml
[sing_box]
start_command = 'sing-box.exe -D . -c 00-local.json -c config.json run'

[subscription]
url = ""
target = "config.json"
# Optional. Defaults to 30 seconds. Valid range: 1..=300.
timeout_secs = 30
```

In this example, `00-local.json` is loaded before `config.json`, so local scalar fields such as `log.level` take priority over the downloaded remote config. Array fields are appended.

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

## Tray Menu

Left-click the tray icon to open the Web UI only when the sing-box core is running.

Right-click for common actions:

- Manage sing-box: start, stop, or restart the core.
- Open the UI and logs.
- Use configuration shortcuts.
- Check, download, or update the sing-box core.
- Toggle administrator mode.
- Toggle startup on login. SingBoost uses a fixed Windows Task Scheduler task name and silently repairs an existing startup task if a portable upgrade moved or renamed the current executable.
- Show About information.
- Exit SingBoost and stop sing-box.

## Logs

SingBoost recreates the following file each time it starts:

```text
<your_app_dir>\logs\singboost-runtime.log
```

Log sources include:

- sing-box standard output and standard error
- SingBoost events and errors

Click Logs in the tray menu to view live log output.
