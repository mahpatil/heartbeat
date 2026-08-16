@../CLAUDE.md

# CLAUDE.md

Guidance for Claude Code when working in this repository.

## What this is

Heartbeat is a Rust service (`heartbeat`) that runs agent/shell tasks defined in markup job files (`.yaml`, `.yml`, `.htb`). It watches `~/.heartbeat/jobs/`, spawns one tokio task per job, and honours cron schedules or one-shot `run_once_at` times.

The Python v0.1.0 codebase is preserved at git tag `v0.1.0`.

## Commands

```bash
# Build
cargo build

# Run (dev)
cargo run

# Tests
cargo test

# Lint / format
cargo clippy
cargo fmt
```

## Source layout

```
src/
├── main.rs, ipc.rs
├── cli/          # subcommands: daemon_cmd, stop, list, apply, install, run_cmd, logs, ipc_client, new/ (wizard)
├── daemon/        # controller.rs, ipc.rs, pid.rs — the running daemon process
├── job/           # config.rs (JobConfig, loads .yaml/.htb), runner.rs, schedule.rs (cron loop)
├── task/          # types.rs (TaskDef enum: Run, Url, FileExists, Agent, AgentApi), executor.rs
├── notify/
└── log/           # writer.rs
```

CLI+daemon architecture (v0.6.0) — daemon runs jobs, CLI talks to it over IPC (`src/ipc.rs`).

## Runtime layout (installed)

```
~/.heartbeat/
├── heartbeat                    # installed binary
├── heartbeat-agent-runner.sh    # shell runner for agent tasks (kept from v0.1.0)
├── jobs/                        # *.yaml / *.htb job files
└── .env                         # secrets loaded at startup
```

## Agent tasks

`agent` tasks delegate to `heartbeat-agent-runner.sh` (resolved from binary dir → `~/.heartbeat/` → `$PATH`). The runner handles `claude`, `opencode`, and `codex`.

## Job file formats

### YAML

```yaml
name: my-check
folder: ~/projects/foo
frequency: "*/5 * * * *"   # cron expression
tasks:
  - type: url
    url: https://example.com
  - type: run
    command: echo hello
  - type: agent
    kind: claude
    prompt: "summarise recent git log"
on_fail:
  - notify-me.sh
```

### Natural language (.htb)

```
# Heartbeat: my-check
# Folder: ~/projects/foo
# Frequency: */5 * * * *

URL reachable: https://example.com
Run: echo hello
Ask Claude: summarise recent git log
```
