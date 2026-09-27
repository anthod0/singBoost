## Rules

- Do not generate or convert the user's `config.json`. The only allowed write to a sing-box config file is an explicit user-triggered remote complete-config download after overwrite confirmation.
- Be conservative with user configuration: critical user-editable configuration is read from `boost.toml`; do not use fallback values for missing or invalid configuration. Only check whether the config file is missing during application startup, and create a default config file if it is missing. Tray-managed application state is stored separately in `boost.state.toml`.
- Core installation or updates must be explicitly triggered and confirmed by the user.
- Keep network, archive, and executable validation work off the UI thread.

## Windows Behavior Notes

- Only relaunch through UAC elevation when `boost.state.toml` has `run_as_admin = true`.
- The sing-box child process inherits the privileges of the current SingBoost process.
- On exit, stop sing-box and close the log window opened by SingBoost.
