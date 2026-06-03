## Overview

SingBoost is a minimal Windows tray launcher for the sing-box core, implemented in Rust.

- Main features: start/stop the colocated `sing-box.exe`, tray menu, log viewing, open Web UI, startup on login, optional administrator privileges, and explicit remote complete-config download with JSON validation/formatting.
- Non-goals: do not generate or convert node configuration; do not bundle the sing-box core.
- Windows is the primary target platform; no non-Windows application is provided.

## Rules

- Do not generate or convert the user's `config.json`. The only allowed write to a sing-box config file is an explicit user-triggered remote complete-config download after overwrite confirmation; validate and pretty-format that downloaded JSON before saving.
- Keep README, AGENTS.md, and the code implementation consistent.
- Be conservative with user configuration: critical user-editable configuration is read from `boost.toml`; do not fallback for missing/invalid configuration. Only check whether the config file is missing during application startup, and create a default config file if it is missing. Tray-managed application state is stored separately in `boost.state.toml`.

## Windows Behavior Notes

- Only relaunch through UAC elevation when `boost.state.toml` has `run_as_admin = true`.
- The sing-box child process inherits the current SingBoost process privileges.
- Use Windows Task Scheduler for startup on login; do not use the registry Run key.
- The scheduled task's "Run with highest privileges" setting should stay consistent with `boost.state.toml` `run_as_admin`.
- On exit, stop sing-box and close the log window opened by SingBoost.

## Verification

```bash
cargo fmt --check
cargo test
```
