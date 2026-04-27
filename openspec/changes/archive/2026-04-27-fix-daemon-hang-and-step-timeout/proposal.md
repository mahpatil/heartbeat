## Why

When a shell step runs an unbounded command (e.g. `mo clean`), the job task hangs indefinitely awaiting `child.wait()`, leaving `status` locked at `Running`. The `list` IPC handler then deadlocks inside the controller event loop, and the CLI has no timeout on its socket read — so `heartbeat list` hangs forever with no feedback to the user.

## What Changes

- `send_ipc` gains a hard deadline (default 5 s, override via `--timeout`) so the CLI fails fast with a clear error when the daemon is unresponsive
- Job step definitions gain an optional `timeout` field (e.g. `timeout: 10m`); shell and agent steps that exceed it have their child process killed and the step marked failed
- The `heartbeat.htb` / YAML job format documentation is updated to show the new `timeout` field

## Capabilities

### New Capabilities
- `ipc-timeout`: CLI-side deadline on all IPC socket reads; surfaces a human-readable error instead of an infinite hang
- `step-timeout`: Per-step optional timeout that kills the child process and marks the step failed when elapsed

### Modified Capabilities
- `ipc-protocol`: The CLI interaction contract changes — callers now receive a timeout error after 5 s if the daemon is unresponsive
- `job-format`: Step definitions gain a new optional `timeout` field

## Impact

- `src/cli/ipc_client.rs` — wrap `send_ipc` inner logic with `tokio::time::timeout`
- `src/task/types.rs` — add `timeout: Option<Duration>` to `StepDef::Shell` and `StepDef::Agent`
- `src/task/executor.rs` — enforce timeout via `tokio::time::timeout` around `child.wait()` / `cmd.status()`; kill child on expiry
- `src/job/config.rs` — parse `timeout` field from YAML (human-readable string → `Duration`)
- No public API changes; no new dependencies required (all primitives are already in `tokio`)
