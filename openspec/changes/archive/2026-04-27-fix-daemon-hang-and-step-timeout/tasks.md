## 1. IPC Client Timeout

- [x] 1.1 Wrap `send_ipc` body in `tokio::time::timeout(Duration::from_secs(5), ...)` in `src/cli/ipc_client.rs`
- [x] 1.2 On timeout, return a clear error: "Daemon not responding (timed out). It may be hung — restart with: heartbeat daemon"
- [x] 1.3 Add unit test: mock a socket that accepts but never writes; assert `send_ipc` returns Err within ~5 s

## 2. Step Timeout — Types & Config Parsing

- [x] 2.1 Add `timeout: Option<Duration>` to `StepDef::Shell` and `StepDef::Agent` in `src/task/types.rs`
- [x] 2.2 Add `timeout` field parsing in `src/job/config.rs` using `humantime::parse_duration`; return parse error if value is invalid
- [x] 2.3 Add `humantime` to `Cargo.toml` if not already a direct dependency (check transitive deps first)
- [x] 2.4 Add unit tests for timeout parsing: valid values (`"10m"`, `"30s"`, `"1h"`), missing (None), invalid ("banana" → error)

## 3. Step Timeout — Execution Enforcement

- [x] 3.1 In `execute_shell` (`src/task/executor.rs`): wrap `child.wait()` with `tokio::time::timeout`; on expiry call `child.kill().await` and return `Err("step timed out after Xs")`
- [x] 3.2 In `execute_agent` (`src/task/executor.rs`): wrap `cmd.status()` with `tokio::time::timeout`; on expiry kill process and return `Err("step timed out after Xs")`
- [x] 3.3 Add unit test: shell step with `timeout: 1s` running `sleep 60` — assert step fails with timeout message within ~2 s
- [x] 3.4 Add unit test: step completes before timeout — assert success and no kill

## 4. Update mo-daily-maintenance Job File

- [x] 4.1 Add `timeout: 5m` to each step in `~/.heartbeat/jobs/mo-daily-maintenance.htb`
- [x] 4.2 Restart the daemon to pick up the changes
