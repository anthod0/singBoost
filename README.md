# SingBoost

SingBoost is a minimal Windows tray launcher for managing `sing-box.exe` in the same directory.

Design source: [`specs/SingBoost-design.md`](specs/SingBoost-design.md)

## Current status

Project scaffold is initialized with:

- Rust binary crate: `singboost`
- Core path derivation helpers for the target deployment layout
- `sing-box check` / `sing-box run` command builders matching the design
- Integration tests for the initialized core behavior

## Development

```bash
cargo fmt --check
cargo check
cargo test
```

## Target deployment layout

```text
D:\Program Files\sing-box\
  singboost.exe
  sing-box.exe
  wintun.dll
  config.json
  logs\
    sing-box.stdout.log
    sing-box.stderr.log
    singboost.log
```
