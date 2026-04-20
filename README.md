# Heartbeat

A lightweight Rust daemon that runs AI agent and shell tasks on a schedule,
from simple markup job files — in your current user context.

**No cron. No root. No credential plumbing.**

Because `heartbeat` is a regular user process (not a system service), it
inherits your full login environment: `$HOME`, `$PATH`, macOS Keychain,
`~/.config/claude/` credentials, API keys. Agent CLIs like `claude`,
`opencode`, and `codex` just work.

> Python v0.1.0 is archived at git tag `v0.1.0`.

---

## Quick start

```bash
# Build from source (Rust required)
cargo build --release
cp target/release/heartbeat ~/.heartbeat/
cp heartbeat-agent-runner.sh ~/.heartbeat/

# Start the daemon (foreground)
heartbeat daemon

# In another terminal — apply a job
cat > /tmp/hello.htb << 'EOF'
---
name: hello
schedule: every 10s
---
echo "heartbeat is running"
EOF

heartbeat apply /tmp/hello.htb

# Check what's running
heartbeat list

# Follow the live log
heartbeat logs hello --follow
```

---

## Job files

Jobs live in `~/.heartbeat/jobs/` as `.htb` files — YAML frontmatter
followed by a free-text prompt body.

### Single-agent job

```
---
name: daily-review
schedule: daily at 02:00
workspace: ~/projects/myapp
agent: claude
flags: [--model claude-opus-4-5]
---
Review the git log from the last 24 hours.
Flag any commits touching auth or payments.
Write a one-paragraph summary to /tmp/daily-review.txt.
```

### Shell job

```
---
name: test-runner
schedule: every 30m
workspace: ~/projects/myapp
agent: shell
---
cargo test 2>&1 | tee /tmp/test-output.txt
```

### Chained steps

```
---
name: nightly-pipeline
schedule: daily at 01:30
workspace: ~/projects/myapp
on_fail:
  - notify-slack.sh "nightly-pipeline failed"
steps:
  - name: run-tests
    type: shell
    command: cargo test 2>&1 | tee /tmp/test-output.txt

  - name: summarise
    type: agent
    agent: claude
    prompt: Read /tmp/test-output.txt and summarise any failures.

  - name: health-check
    type: url-check
    url: https://myapp.example.com/health
---
```

### Frontmatter fields

| Field | Required | Default | Description |
|---|---|---|---|
| `name` | yes | — | Unique job identifier |
| `schedule` | yes | — | When to run (see below) |
| `workspace` | no | `~` | Working directory |
| `agent` | no | `claude` | Default agent for body-as-prompt |
| `flags` | no | `[]` | Extra CLI flags passed to the agent |
| `on_fail` | no | `[]` | Shell commands to run on failure |
| `steps` | no | — | Explicit multi-step pipeline |

---

## Schedule syntax

```
every 30s          # every 30 seconds
every 5m           # every 5 minutes
every 2h           # every 2 hours
every 1d           # every day

daily at 02:00     # once per day at 02:00 local time
daily at 14:30

once at 2026-06-01T09:00:00Z   # one-shot RFC-3339
once at 14:00                  # one-shot, next 14:00 today/tomorrow
```

Intervals use **Delay** behaviour — if a job overruns, the next fire is
counted from when the previous run finished, not from a backlog of missed ticks.

---

## Supported agents

| Agent | CLI invoked |
|---|---|
| `claude` | `claude -p <prompt> [flags]` |
| `opencode` | `opencode run <prompt> [flags]` |
| `codex` | `codex exec <prompt> [flags]` |
| `shell` | `bash -c <prompt>` |
| `<custom>` | `<custom> <prompt> [flags]` |

All agent invocations go through `heartbeat-agent-runner.sh`, which enriches
`$PATH` (adds Homebrew, `~/.local/bin`, etc.) before calling the CLI.

---

## CLI reference

```
heartbeat daemon                  # start the daemon (foreground)
heartbeat apply <file.htb>        # install a job (hot-reload, no restart needed)
heartbeat list                    # show all jobs: name, status, schedule, next run
heartbeat run <name>              # trigger an immediate run
heartbeat stop <name>             # stop a job (removed from scheduler)
heartbeat logs <name>             # print current log
heartbeat logs <name> --follow    # tail -F the live log
heartbeat install --autostart     # write ~/Library/LaunchAgents/com.heartbeat.plist
```

All `heartbeat <cmd>` calls (except `daemon`) talk to the running daemon via a
Unix socket at `~/.heartbeat/heartbeat.sock`.

---

## Runtime layout

```
~/.heartbeat/
├── heartbeat                    # binary
├── heartbeat-agent-runner.sh    # agent execution wrapper
├── .env                         # secrets loaded at daemon start
├── heartbeat.pid                # daemon PID (present while running)
├── heartbeat.sock               # Unix IPC socket
├── jobs/
│   └── *.htb                    # job files (hot-watched; no restart needed)
└── logs/
    ├── <name>.log               # current log (10 MB max)
    └── <name>.log.1 … .5        # rotated logs
```

---

## Logs

Each job writes timestamped lines to `~/.heartbeat/logs/<name>.log`:

```
2026-04-21T02:00:00Z [daily-review] ===== run started =====
2026-04-21T02:00:01Z [daily-review] [step[0]] agent=claude workspace=/Users/me/projects/myapp
2026-04-21T02:00:44Z [daily-review] ===== run completed (44s) =====
```

Shell steps stream stdout/stderr line-by-line as the command runs.
Agent steps write through the runner's `-l` flag directly to the log.

---

## Building

Requires Rust 1.75+.

```bash
cargo build           # debug
cargo build --release # optimised (~2-3 MB after strip)
cargo test            # 55 unit tests
```

---

## Roadmap

| Milestone | Status |
|---|---|
| 1 — Core daemon (schedule, executor, hot-reload) | ✅ |
| 2 — Control plane (IPC socket, CLI commands, log rotation) | ✅ |
| 3 — Chained steps (multi-agent pipelines) | ✅ |
| 4 — Distribution (`install.sh`, pre-built binaries, LaunchAgent) | 🔲 |

Full specs: [`openspec/roadmap.md`](openspec/roadmap.md)
