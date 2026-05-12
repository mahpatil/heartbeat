# 💓 heartbeat - lightweight scheduler for all things AI or non-AI

<p align="center">
  <img src="https://img.shields.io/badge/rust-v1.75+-orange.svg" alt="Rust">
  <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License">
  <img src="https://img.shields.io/badge/build-passing-brightgreen.svg" alt="Build">
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg" alt="Platform">
</p>

<p align="center">
  <b>Lightweight AI-agent and shell task scheduler for minimalists.</b>
</p>
<p align="center">
  <b>No cron. No root. No credential plumbing.</b>
</p>

<p align="center">
  <code>__/\__/\__/\__/\__/\__/\__/\__/\__/\__/\__/\__</code>
</p>

Heartbeat is a zero-config Rust daemon that runs your AI agent and shell tasks on a schedule, directly in your user context. No root, no complex credential plumbing, no `cron` headaches. It just works with the environment you already have. Run it in your personal context, in a sandbox, or inside a container.

---

## ⚡ Why Heartbeat?

Modern automation shouldn't require setting up a heavy orchestration engine or wrestling with system-level cron jobs that lack your environment variables and API keys.

| Feature | `heartbeat` | `cron` | `Airflow` / `Dagster` | `OpenClaw` |
| :--- | :---: | :---: | :---: | :---: |
| **Setup Time** | < 1 min | Quick | Hours | Quick |
| **User Environment** | ✅ Native | ❌ Stripped | ❌ Containerized | ✅ Native |
| **Agent Friendly** | ✅ Built-in | ❌ No | ❌ Complex |  ✅ Built-in |
| **Multi-step Pipes** | ✅ YAML | ❌ Scripting | ✅ Python DSL | ✅ Skills |
| **Dependencies** | None (Binary) | System | Many | Many |
| **Footprint** | Minimal | Small | Large | Large |

---

## 🚀 Quick Start

### 1. Install
```bash
# Binary + daemon
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash

# Binary + /heartbeat Claude Code skill
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash -s -- --claude-skill

# Binary + opencode skill
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash -s -- --opencode-skill

# Binary + codex skill
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash -s -- --codex-skill

# All skills at once
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash -s -- --claude-skill --opencode-skill --codex-skill
```

### 2. Start the Daemon
```bash
heartbeat daemon
```

### 3. Schedule a Job
Create a `.htb` file (YAML frontmatter + body) and apply it:
```bash
cat > hello.htb << 'EOF'
---
name: hello-world
schedule: every 10s
---
echo "Heartbeat is alive at $(date)"
EOF

heartbeat apply hello.htb
```

### 4. Monitor
```bash
heartbeat list              # See what's running
heartbeat logs hello-world  # Tail the logs
```

---

## 5. Building from source (optional)

Requires Rust 1.75+.

```bash
cargo build           # debug
cargo build --release # optimised (~2-3 MB after strip)
cargo test            # 92 Rust unit tests + 15 shell tests
```

Shell tests (no network required):
```bash
bash tests/install/test_install_sh.sh
```


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

## 🛠️ Key Features

- **🤖 Agent Native:** Built-in support for `claude`, `opencode`, and `codex`. Run AI agents as scheduled tasks.
- **🔄 Hot Reload:** Just edit your `.htb` files in `~/.heartbeat/jobs/`. The daemon picks up changes instantly without a restart.
- **🛡️ Native Security:** Inherits your `$HOME`, `$PATH`, and macOS Keychain. Your API keys and credentials are automatically available.
- **🔗 Chained Steps:** Create complex pipelines with multiple steps, branching on failure, and URL health checks.
- **📦 Zero Config:** No database, no root access, no global configuration files. Everything lives in `~/.heartbeat/`.

---

## 🧩 Architecture

```text
    [ User CLI ] <----(Unix Socket)----> [ heartbeat daemon ]
         ^                                     |
         |                                     |--- [ Watcher ] (Hot-reloads .htb)
         |                                     |--- [ Scheduler ] (Timing logic)
    [ .htb Job ] ----------------------------> |--- [ Executor ] (Runs Shell/Agents)
```

---

## 🤖 Agent Skills

Heartbeat integrates directly into your favorite AI agent CLIs. Schedule jobs using natural language directly from your chat interface:

*   **Claude Code:** `/heartbeat review my code every night at 2am`
*   **OpenCode:** `schedule run tests every 30 minutes`
*   **Codex:** `schedule check health every 5 minutes`

Install skills with: `heartbeat install --claude-skill`

---

## 📊 Reliability & Stats

Heartbeat is built for rock-solid reliability in Rust.

- **115+** Unit & Integration tests.
- **< 3MB** Binary footprint (macOS).
- **Zero** external runtime dependencies.
- **Automatic** log rotation (10MB max per job).


---

## Quick start

```bash
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

## Creating jobs

### Interactive wizard (humans)

```bash
heartbeat new           # launches a step-by-step wizard
heartbeat new --apply   # wizard + immediately deploy to daemon
```

### Non-interactive flags (agents / CI)

All parameters can be passed as flags — no TTY required. When `--name`,
`--schedule`, and `--prompt` are all provided the wizard is skipped entirely.

```bash
heartbeat new \
  --name daily-digest \
  --schedule "every 1h" \
  --prompt "/reviewer" \
  --agent claude \
  --workspace ~/projects/myapp \
  --flags=--dangerously-skip-permissions \
  --apply
```

| Flag | Required | Default | Description |
|---|---|---|---|
| `--name` | yes* | — | Job slug |
| `--schedule` | yes* | — | Schedule string |
| `--prompt` | yes* | — | Agent prompt (or skill, e.g. `/reviewer`) |
| `--agent` | no | `claude` | Agent to use |
| `--workspace` | no | `~` | Working directory |
| `--flags` | no | — | Extra agent flags (repeatable, e.g. `--flags=--dangerously-skip-permissions`) |
| `--output` | no | `<name>.htb` | Output file path |
| `--apply` | no | false | Also deploy to daemon |

\* All three required together to skip the wizard.

---

## CLI reference

```
heartbeat daemon                    # start the daemon (foreground)
heartbeat new                       # create a job (interactive wizard)
heartbeat new --name … --apply      # create a job (non-interactive, agent-friendly)
heartbeat apply <file.htb>          # deploy an existing job file (hot-reload)
heartbeat list                      # show all jobs: name, status, schedule, next run
heartbeat run <name>                # trigger an immediate run
heartbeat stop <name>               # stop a job (removed from scheduler)
heartbeat logs <name>               # print current log
heartbeat logs <name> --follow      # tail -F the live log
heartbeat install --autostart       # write ~/Library/LaunchAgents/com.heartbeat.plist
heartbeat install --claude-skill    # install /heartbeat slash command for Claude Code
heartbeat install --opencode-skill  # install heartbeat skill for opencode
heartbeat install --codex-skill     # install heartbeat skill for codex
heartbeat uninstall --autostart     # remove LaunchAgent
heartbeat uninstall --opencode-skill  # remove opencode skill
heartbeat uninstall --codex-skill     # remove codex skill
```

All `heartbeat <cmd>` calls (except `daemon` and `new`) talk to the running daemon via a
Unix socket at `~/.heartbeat/heartbeat.sock`.

---

## Native agent skills

Install the heartbeat skill for whichever agent(s) you use. Each lets you schedule jobs using natural language directly from your agent's chat interface.

```bash
heartbeat install --claude-skill    # Claude Code → /heartbeat <description>
heartbeat install --opencode-skill  # opencode → schedule <description>
heartbeat install --codex-skill     # codex → schedule <description>
```

### Claude Code (`/heartbeat`)

```
/heartbeat review auth code every night at 2am in ~/projects/myapp
/heartbeat run /reviewer on this repo every hour
/heartbeat check https://myapp.com/health every 5 minutes with opencode
/heartbeat summarise git log daily using codex in ~/projects/backend
/heartbeat implement the spec in openspec/features/payments.md at 9am
```

### opencode / codex (heartbeat skill)

Once installed, the skill is automatically available. Just describe what you want scheduled in plain English — the agent translates it to a `heartbeat new` call:

```
schedule review auth code every night at 2am in ~/projects/myapp
schedule run tests every 30 minutes
schedule check https://myapp.com/health every 5 minutes
schedule summarise git log daily in ~/projects/backend
```

All three skills infer the job name, schedule, agent, and workspace from plain English
and call `heartbeat new` non-interactively — no wizard, no TTY required.

**Agent inference:**

| You say | Agent used | Extra flags |
|---|---|---|
| _(default)_ or "with claude" | `claude` | `--dangerously-skip-permissions` |
| "with opencode" / "using opencode" | `opencode` | _(none)_ |
| "with codex" / "using codex" | `codex` | _(none)_ |

---

## Using Claude Code skills inside jobs

Claude Code **skills** (slash commands like `/coder`, `/reviewer`) can be used
as the `prompt` in any agent step — running them on a schedule or as pipeline stages.

### Single-skill job

```
---
name: nightly-review
schedule: daily at 23:00
workspace: ~/projects/myapp
agent: claude
flags: ["--dangerously-skip-permissions"]
---
/reviewer
```

### Skills as pipeline steps

```
---
name: spec-then-code
schedule: once at 2026-05-01T09:00:00Z
workspace: ~/projects/myapp
steps:
  - name: generate-spec
    type: agent
    agent: claude
    prompt: "/spec openspec/issues/42-auth-redesign.md"
    flags: ["--dangerously-skip-permissions"]

  - name: implement
    type: agent
    agent: claude
    prompt: "/coder openspec/issues/42-auth-redesign.md"
    flags: ["--dangerously-skip-permissions"]

  - name: commit
    type: agent
    agent: claude
    prompt: "/git-commit"
    flags: ["--dangerously-skip-permissions"]
---
```
---

## Troubleshooting

### `heartbeat` is immediately killed — even `heartbeat --help`

macOS Gatekeeper or Sequoia's stricter execution policy can SIGKILL an unsigned
binary before it runs a single instruction. This affects binaries copied manually
(e.g. `cp target/release/heartbeat ~/.heartbeat/`) without going through the
installer.

**Fix:**

```bash
# Strip quarantine attributes and ad-hoc sign the binary
xattr -c ~/.heartbeat/heartbeat
codesign --sign - --force ~/.heartbeat/heartbeat

# Verify it works
heartbeat --version
```

The installer (`install.sh`) does this automatically for every install path
(pre-built download and cargo build). If you copy the binary manually after a
`cargo build --release`, run the two commands above.

---

## 🗺️ Roadmap
| Milestone | Status |
|---|---|
| 1 — Core daemon (schedule, executor, hot-reload) | ✅ |
| 2 — Control plane (IPC socket, CLI commands, log rotation) | ✅ |
| 3 — Chained steps (multi-agent pipelines) | ✅ |
| 4 — Distribution (`install.sh`, pre-built binaries, LaunchAgent) | ✅ |
| 5 — Interactive Job Wizard CLI (`heartbeat new`, non-interactive agent flags) | ✅ |

- [ ] Distributed workers (experimental)
- [ ] Web Dashboard (read-only status)
- [ ] Desktop Notifications on failure

---

## 📄 License

Distributed under the MIT License. See `LICENSE` for more information.

---

<p align="center">
  Built with ❤️ for the AI automation community.
</p>
