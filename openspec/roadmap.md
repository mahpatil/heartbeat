# Heartbeat — Roadmap

Heartbeat is a lightweight Rust daemon that runs agent and shell tasks from
prompt markup files (`.htb`) in the current user's login context. No cron.
No root. Credentials (Claude, OpenCode, Codex, API keys) are naturally
available because the daemon is a user process, not a system service.

---

## Milestone 1 — Core Service (MVP)

Get a working daemon that can schedule and execute simple jobs.

| # | Spec | Description | Status |
|---|------|-------------|--------|
| 1 | [core-daemon](specs/core-daemon/spec.md) | Long-running tokio daemon: job registry, scheduler loop, graceful shutdown | ✅ |
| 2 | [job-format](specs/job-format/spec.md) | `.htb` frontmatter + prompt body file format | ✅ |
| 3 | [schedule-engine](specs/schedule-engine/spec.md) | Human-friendly schedules: `every Nm`, `daily at HH:MM`, `once at` | ✅ |
| 4 | [agent-executor](specs/agent-executor/spec.md) | Execute claude/opencode/codex/shell via heartbeat-agent-runner.sh in user context | ✅ |

## Milestone 2 — Control Plane

Manage running jobs without restarting the daemon.

| # | Spec | Description | Status |
|---|------|-------------|--------|
| 5 | [ipc-protocol](specs/ipc-protocol/spec.md) | Unix socket IPC — list, run, stop, reload over line-delimited JSON | ✅ |
| 6 | [cli-commands](specs/cli-commands/spec.md) | `heartbeat` CLI: daemon, apply, list, run, stop, logs | ✅ |
| 7 | [log-system](specs/log-system/spec.md) | Per-job structured log files with rotation, streamed in real time | ✅ |

## Milestone 3 — Power Features

Multi-step pipelines and agent chaining.

| # | Spec | Description | Status |
|---|------|-------------|--------|
| 8 | [chained-steps](specs/chained-steps/spec.md) | Sequential multi-step jobs: agent → shell → url-check in one file | ✅ |

## Milestone 4 — Distribution

Make heartbeat installable without Rust.

| # | Spec | Description | Status |
|---|------|-------------|--------|
| 9  | [install-distribution](specs/install-distribution/spec.md) | `install.sh`: download pre-built binary, verify checksum, configure PATH | 🔲 |
| 10 | [autostart-launchagent](specs/autostart-launchagent/spec.md) | `heartbeat install --autostart`: macOS LaunchAgent for login persistence | 🔲 |

---

## Status Key

| Symbol | Meaning |
|--------|---------|
| 🔲 | Not started |
| 🔄 | In progress |
| ✅ | Complete |
| ⏸ | Deferred |
