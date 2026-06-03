## Overview

SingBoost is a minimal Windows tray launcher for the sing-box core, implemented in Rust.

- Main features: start/stop the colocated `sing-box.exe`, tray menu, log viewing, open Web UI, startup on login, and optional administrator privileges.
- Non-goals: do not generate, modify, or parse node configuration; do not bundle the sing-box core.
- Windows is the primary target platform; no non-Windows application is provided.

## Rules

- Do not generate, modify, or rewrite the user's `config.json`.
- Keep README, AGENTS.md, and the code implementation consistent.
- Be conservative with user configuration: critical application configuration is read from `boost.toml`; do not fallback for missing/invalid configuration. Only check whether the config file is missing during application startup, and create a default config file if it is missing.

## Windows Behavior Notes

- Only relaunch through UAC elevation when `app.run_as_admin = true`.
- The sing-box child process inherits the current SingBoost process privileges.
- Use Windows Task Scheduler for startup on login; do not use the registry Run key.
- The scheduled task's "Run with highest privileges" setting should stay consistent with `app.run_as_admin`.
- On exit, stop sing-box and close the log window opened by SingBoost.

## Verification

```bash
cargo fmt --check
cargo test
```
