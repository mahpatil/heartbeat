## Why

`heartbeat install --autostart` already fully implements launchd registration
(writes plist + calls `launchctl load -w`, idempotent, unload on reinstall).
The daemon simply isn't registered because nothing prompts the user to run it.
After a reboot or terminal close the daemon silently stops and jobs don't run.
The fix is discoverability: `heartbeat daemon` and `heartbeat list` should
detect the missing LaunchAgent and suggest the one-liner to fix it.

## What Changes

- `heartbeat daemon` prints a one-time notice on startup when no LaunchAgent
  plist exists: "Tip: run `heartbeat install --autostart` to start automatically at login"
- `heartbeat list` (and other IPC commands) include an `autostart: false` hint
  in their output when the plist is absent, reminding the user
- No functional changes to `install --autostart` — it already works correctly

## Capabilities

### New Capabilities
_(none — launchd integration is already implemented)_

### Modified Capabilities
- `autostart-launchagent`: Add the "not registered" warning to daemon startup
  and to `heartbeat list` output so users discover `--autostart` without reading docs
- `cli-commands`: `heartbeat daemon` and `heartbeat list` gain the hint message

## Impact

- `src/cli/daemon_cmd.rs` — print the autostart hint on startup if plist absent
- `src/cli/list.rs` — append autostart hint line when plist not found
- No new dependencies; `std::path::Path::exists()` check is sufficient
