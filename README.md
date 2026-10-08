# SingBoost

A minimal Windows launcher for the sing-box core.

## Features

- Windows system tray icon
- Start or stop the sing-box core
- Open the sing-box Web UI
- Configure startup on login
- Download a complete remote sing-box JSON config on demand
- Download or update the sing-box core from official stable releases

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

### Basic Auth

For a subscription that requires HTTP Basic Auth, set both credentials in `boost.toml`:

```toml
[subscription]
url = "https://example.com/config.json"
target = "config.json"
username = "your-username"
password = "your-password"
```

Omit both fields for anonymous downloads. Providing only one is an error. The username must be non-empty and cannot contain `:`; an empty password is allowed. Credentials are stored as plain text in `boost.toml`. Use HTTPS: Basic Auth does not encrypt credentials.

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
