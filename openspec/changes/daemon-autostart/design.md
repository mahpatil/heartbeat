## Context

`heartbeat install --autostart` is fully functional: it writes the LaunchAgent
plist and calls `launchctl load -w`. The gap is discoverability — users run
`heartbeat daemon` manually and never discover `--autostart` exists. After a
reboot or terminal close the daemon stops silently.

## Goals / Non-Goals

**Goals:**
- `heartbeat daemon` warns once on startup if no LaunchAgent is registered
- `heartbeat list` shows a hint line when the plist is absent
- Zero friction: the hint is a complete, copy-pasteable one-liner

**Non-Goals:**
- Changing `install --autostart` behaviour (already correct)
- Auto-installing the LaunchAgent without user consent
- Cross-platform autostart (Linux systemd, Windows services)

## Decisions

### D1 — Hint on `heartbeat daemon` startup, not as an error

Print to stderr as an `INFO` log line, not as a warning or error. The daemon
starts successfully either way; this is advisory.

```
INFO heartbeat daemon starting
INFO Tip: run `heartbeat install --autostart` to start automatically at login
```

Only printed when `~/Library/LaunchAgents/com.heartbeat.plist` does not exist.

### D2 — Hint line at bottom of `heartbeat list` output

After the job table, if plist is absent:

```
NAME    STATUS  SCHEDULE       NEXT RUN
─────────────────────────────────────────
my-job  idle    daily at 02:00 ...

Tip: daemon will not survive reboot — run `heartbeat install --autostart`
```

Keeps the happy-path table clean while surfacing the hint where users look.

### D3 — plist path check is a pure filesystem stat

`install::plist_path(home)` already returns the canonical path. Reuse it in
both `daemon_cmd.rs` and `list.rs` with a simple `Path::exists()` check.
No IPC, no launchctl invocation needed for the check.

## Risks / Trade-offs

- **Hint noise if user intentionally skips autostart** → Low risk; easily
  silenced by running `--autostart` once. Could add `--no-autostart-hint` flag
  later if this becomes annoying.
- **macOS-only check** → Acceptable; heartbeat currently only ships launchd
  support. Linux users don't see the hint since the plist path won't exist.

## Migration Plan

1. Add hint to `daemon_cmd.rs`.
2. Add hint to `list.rs`.
3. Ship. User runs `heartbeat install --autostart` once and hints disappear.
