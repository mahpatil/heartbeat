# autostart-launchagent Specification

## Purpose

Allow heartbeat to start automatically at user login on macOS without
requiring manual `heartbeat daemon` invocation. Uses a macOS LaunchAgent
(NOT LaunchDaemon) so the process runs in the user's GUI session, with
full access to the Keychain, `~/.config/`, and all user credentials.

---

## Requirements

### Requirement: LaunchAgent vs LaunchDaemon distinction

The auto-start mechanism SHALL use a LaunchAgent installed in
`~/Library/LaunchAgents/`, never a LaunchDaemon in `/Library/LaunchDaemons/`.

LaunchAgents run as the logged-in user in the GUI session:
- Full Keychain access (claude, opencode credentials)
- `$HOME` is set correctly
- User-installed binaries in PATH are accessible via PATH enrichment in the runner

LaunchDaemons run as root at system boot with no user session — incompatible
with agent credential access.

#### Scenario: plist is a LaunchAgent
- GIVEN `heartbeat install --autostart` is run
- WHEN the plist is written
- THEN it is at `~/Library/LaunchAgents/com.heartbeat.plist`
- AND it does NOT appear in `/Library/LaunchDaemons/`

---

### Requirement: plist contents

The generated plist SHALL:
- Set `Label` to `com.heartbeat`
- Set `ProgramArguments` to `["/Users/<user>/.heartbeat/heartbeat", "daemon"]`
- Set `RunAtLoad` to `true`
- Set `KeepAlive` to `true` (restart if it crashes)
- Set `StandardOutPath` and `StandardErrorPath` to `~/.heartbeat/logs/daemon.log`
- Set `EnvironmentVariables` to a dict containing `HOME` and the user's current `PATH`

#### Scenario: plist written correctly
- GIVEN `heartbeat install --autostart` is run as user `mahesh`
- WHEN the plist is generated
- THEN `ProgramArguments` is `["/Users/mahesh/.heartbeat/heartbeat", "daemon"]`
- AND `EnvironmentVariables.HOME` is `/Users/mahesh`

---

### Requirement: Automatic load after install

After writing the plist, `heartbeat install --autostart` SHALL call
`launchctl load ~/Library/LaunchAgents/com.heartbeat.plist` to start
the agent in the current session without requiring a logout.

#### Scenario: Daemon starts immediately
- GIVEN `heartbeat install --autostart` completes
- WHEN `heartbeat list` is run immediately after
- THEN the daemon is reachable (socket exists)

---

### Requirement: `heartbeat install --autostart` is idempotent

If the plist already exists and the agent is already loaded, running
`heartbeat install --autostart` again SHALL:
1. Unload the existing agent (`launchctl unload`).
2. Overwrite the plist with fresh content.
3. Reload the agent (`launchctl load`).

#### Scenario: Re-run after binary update
- GIVEN heartbeat was already installed with --autostart
- WHEN the binary is updated and `heartbeat install --autostart` is run again
- THEN the daemon restarts with the new binary

---

### Requirement: `heartbeat uninstall --autostart`

A corresponding uninstall command SHALL:
1. Unload the LaunchAgent (`launchctl unload`).
2. Delete the plist file.
3. Print "Removed LaunchAgent. heartbeat will no longer start at login."

#### Scenario: Clean removal
- GIVEN the LaunchAgent is loaded
- WHEN `heartbeat uninstall --autostart` is run
- THEN the agent is stopped
- AND `~/Library/LaunchAgents/com.heartbeat.plist` is deleted
- AND `heartbeat list` reports "daemon not running"

---

### Requirement: Daemon log via LaunchAgent

When running as a LaunchAgent (non-interactive), the daemon output SHALL
go to `~/.heartbeat/logs/daemon.log` (configured via `StandardOutPath` in
the plist). This file follows the same rotation rules as per-job logs
(10 MB max, 5 rotations — see log-system spec).
