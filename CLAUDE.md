# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run all tests
python3 -m pytest tests/ -v

# Run a single test
python3 -m pytest tests/test_heartbeat.py::TestTaskRunner::test_run_shell_success -v

# Run with coverage
python3 -m pytest tests/ --cov=heartbeat --cov-report=term-missing

# Lint
ruff check .

# Run a job manually (requires installed ~/.heartbeat setup)
heartbeat run <job-name>

# Run heartbeat.py directly with a config file
python3 heartbeat.py -c jobs/sample-check.yaml
```

## Architecture

The project has two entry points:

- **`heartbeat.py`** — Python core: parses configs, runs tasks, manages `run_once_at` logic, sends notifications
- **`heartbeat`** — Bash CLI wrapper: manages job files in `~/.heartbeat/jobs/`, adds/removes crontab entries tagged `# HEARTBEAT`
- **`heartbeat-agent-runner.sh`** — Can also be called directly from cron (no config file needed) for single-agent prompts: `heartbeat-agent-runner.sh [-l logfile] <agent> <cwd> <prompt> [params...]`

### Core classes in `heartbeat.py`

- **`Heartbeat`** — Orchestrates a job run. Loads config, checks `run_once_at` gate, iterates tasks, calls `on_fail` commands, sends notifications, and prints `REMOVE_CRON:<name>` to stdout if a `run_once_at` job completes (the bash CLI watches for this signal to remove the cron entry).
- **`TaskRunner`** — Executes individual tasks by type: `run` (shell — routed through `heartbeat-agent-runner.sh` for cron-safe HOME/PATH), `url` (HTTP check), `file_exists`, `agent`/`claude`/`opencode`/`codex` (delegates to `heartbeat-agent-runner.sh`), `agent_api` (calls Anthropic or OpenAI SDK directly). All shell and agent execution goes through the runner; `folder` defaults to `~` when unset.
- **`ConfigParser`** — Routes `.yaml`/`.yml` files to YAML parsing and `.htb` files to the natural-language parser (`_parse_nl`).
- **`parse_simple_yaml()`** — Stdlib-only fallback YAML parser used when PyYAML is not installed.

### Plugin system (`plugins/`)

Notification plugins extend `BasePlugin` (ABC in `plugins/base.py`). Register new plugins in `plugins/__init__.py`'s `PLUGINS` dict. The `env:VAR_NAME` syntax in config values is resolved by `BasePlugin._get_env_var()`.

### Job config formats

**YAML** (`.yaml`/`.yml`) — top-level fields: `name`, `folder`, `frequency` (cron expression), `run_once_at` (ISO datetime), `tasks` (list), `notifications` (list).

**Natural language** (`.htb`) — header comments `# Heartbeat:`, `# Folder:`, `# Frequency:`, `# Run once at:`; tasks listed under `Every N minutes:` block with keys like `URL reachable:`, `File exists:`, `Run:`, `Ask Claude:`.

### Runtime file layout (installed)

```
~/.heartbeat/
├── heartbeat.py      # Installed core script
├── heartbeat         # Installed CLI
├── heartbeat-agent-runner.sh  # Installed agent CLI runner
├── jobs/             # Job configs (*.yaml, *.htb)
├── logs/             # Per-job log files (<name>.log)
└── plugins/          # Notification plugins
```

The repo itself is the development source; `install.sh` downloads files into `~/.heartbeat/` and adds it to `$PATH`.
