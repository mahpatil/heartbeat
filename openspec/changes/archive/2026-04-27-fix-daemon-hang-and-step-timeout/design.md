## Context

`heartbeat list` hangs indefinitely when the daemon is alive but its tokio
runtime is not processing IPC requests. Root cause: a shell step (`mo clean`)
ran without a timeout, blocking the job's `run_once` in `child.wait()`.
The `List` IPC handler then stalled the controller event loop awaiting
`status.lock()` — leaving the entire daemon unresponsive.

Two independent problems need fixing:

1. **CLI has no IPC deadline.** `send_ipc` calls `read_line().await` with no
   timeout. If the daemon stops responding for any reason, every CLI command
   hangs forever with no feedback.

2. **Steps have no execution budget.** Shell and agent steps can run
   indefinitely. A stuck subprocess blocks the job task and, indirectly, the
   controller.

## Goals / Non-Goals

**Goals:**
- `heartbeat list` (and all CLI commands) fail fast with a clear error when
  the daemon is unresponsive
- Steps that exceed an optional `timeout` field are killed and marked failed
- No new dependencies; all primitives already exist in tokio

**Non-Goals:**
- Automatically restarting a hung daemon
- Global (per-job) timeout — this is per-step only
- Configuring the IPC deadline per-invocation (a fixed default is sufficient)

## Decisions

### D1 — Fixed 5 s IPC deadline, not configurable

Wrap the entire `send_ipc` body with `tokio::time::timeout(Duration::from_secs(5), ...)`.
A 5 s deadline is generous for any local Unix socket operation. Making it
configurable (flag or env var) adds complexity with minimal benefit; the
common case is either "daemon responds instantly" or "daemon is hung".

_Alternative considered_: per-command `--timeout` flag. Rejected — adds CLI
surface area for a problem users should never see if step timeouts are in place.

### D2 — Step timeout kills the child process then returns Err

In `execute_shell` and `execute_agent`, wrap `child.wait()` / `cmd.status()`
with `tokio::time::timeout(step_timeout, ...)`. On expiry:
1. Call `child.kill().await` (sends SIGKILL)
2. Return `Err("step timed out after Xs")`

This propagates through `run_once` → `JobStatus::Failed` → `on_fail` hooks.
The job loop continues on its normal schedule next tick.

_Alternative considered_: SIGTERM first, then SIGKILL after a grace period.
Rejected — adds complexity; shell tools that run `mo clean` are not expected
to handle signals gracefully anyway.

### D3 — Timeout parsed from human-readable duration string

YAML field accepts values like `"10m"`, `"30s"`, `"1h"`. Parse with the
`humantime` crate (already a transitive dependency via `tracing-subscriber`).

```yaml
steps:
  - name: clean
    type: shell
    command: mo clean
    timeout: 10m
```

_Alternative considered_: seconds-as-integer. Rejected — harder to read and
write for multi-minute timeouts.

### D4 — Timeout field is optional; absence means no limit (current behaviour)

Existing jobs without `timeout` continue to work exactly as before. No
migration needed.

## Risks / Trade-offs

- **Timeout too short kills legitimate long-running steps** → Mitigation: no
  default timeout; users opt in per-step. Document recommended values.
- **SIGKILL leaves child processes with open file handles** → Mitigation: this
  is the same behaviour as the current `handle.abort()` path; acceptable for
  a maintenance tool.
- **IPC 5 s deadline may be too short on a heavily loaded machine** → Risk is
  low; local Unix socket round-trips are sub-millisecond. If the daemon takes
  >5 s to respond it is effectively hung.

## Migration Plan

1. Deploy the updated binary — existing job files without `timeout` are
   unaffected.
2. Add `timeout:` to any steps known to run long (e.g. `mo clean → timeout: 5m`).
3. Restart the daemon to pick up the new binary.

No rollback concerns — the `timeout` field is ignored by old binaries (YAML
unknown fields are currently silently skipped).
