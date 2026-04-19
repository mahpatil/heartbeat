# Heartbeat

A simple scheduled task runner with config as code. Runs tasks on a cron schedule, checks URLs, files, and can invoke AI agents (Claude, OpenCode, Codex).

## Features

- **Multiple formats**: YAML or natural language (.htb) config
- **Task types**: Run shell commands, check URLs, check file existence
- **Agent support**: Invoke Claude/OpenCode/Codex CLI or API
- **One-time scheduling**: Run once at a specific time (`run_once_at`)
- **Notifications**: Telegram plugin for success/failure alerts
- **Easy cron setup**: Natural language frequency parsing
- **Zero deps**: Python 3 stdlib only

## Requirements

- Python 3.x (pre-installed on macOS/Linux)
- Optional: `claude`, `opencode`, or `codex` CLI for agent tasks
- Optional: `anthropic` or `openai` Python packages for API mode
- Optional: Telegram bot token for notifications

## Quick Start

```bash
# 1. Install
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash

# 2. Create a job
mkdir -p ~/.heartbeat/jobs
cat > ~/.heartbeat/jobs/my-check.yaml << 'EOF'
name: "my-check"
folder: "~/project"
frequency: "*/15 * * * *"

tasks:
- type: url
  url: "https://example.com"
  on_fail: "echo 'Site down!'"
  
- type: run
  command: "echo 'Check ran at $(date)'"
EOF

# 3. Run manually
heartbeat run my-check

# 4. Add to cron (automatic scheduling)
heartbeat add-cron my-check

# 5. View cron entries
crontab -l
```

## Usage

```bash
# Create a new job (prompts to edit config and add cron separately)
heartbeat add "job name" --folder ~/project --frequency "every 15 min"

# Create job from existing config template
heartbeat add "job name" --config /path/to/existing.yaml

# Create job and add to cron in one step
heartbeat add "job name" --folder ~/project --frequency "15min" --cron

# Create job from template and add to cron
heartbeat add "job name" --config /path/to/existing.yaml --cron

heartbeat add-cron "job name"    # Add to crontab
heartbeat remove-cron "job name" # Remove only cron entry
heartbeat list                   # List all jobs
heartbeat run "job name"         # Run manually
heartbeat logs [name]            # Show logs
heartbeat remove "job name"      # Remove job and cron
```

## Config Format

### YAML (`.yaml`)

```yaml
name: "my-job"
folder: "~/project"
frequency: "*/15 * * * *"

tasks:
- type: run
  command: "echo 'hello'"

- type: url
  url: "https://example.com"
  on_fail: "echo 'Failed!'"

- type: file_exists
  path: "data.json"

- agent: claude
  

- agent: opencode
  
  prompt: "Check for bugs in this folder"

- agent_api:
  provider: anthropic
  model: claude-sonnet-4-20250514
  prompt: "Summarize this file"

- type: telegram
  api_token: env:TELEGRAM_BOT_TOKEN
  chat_id: env:TELEGRAM_CHAT_ID
  on_failure: true
  on_success: false
```

#### One-Time Scheduling

Run a job exactly once at a specific time:

```yaml
name: "my-once-job"
folder: "~/project"
frequency: "*/15 * * * *"
run_once_at: "2026-04-15 09:00"

tasks:
- type: run
  command: "echo 'Morning report'"
```

**How it works:**
1. Add a `run_once_at: "YYYY-MM-DD HH:MM"` field to your job config
2. Add the job to cron: `heartbeat add-cron my-once-job`
3. When the scheduled time is reached, the job runs
4. After successful completion, the cron entry is automatically removed

**Notes:**
- The job config must have a `frequency` field (used by cron to schedule when to check)
- The `run_once_at` time uses local timezone
- Logs track completion with a marker to prevent re-running
- Both the `run_once_at` key and `# Run once at:` comment are supported

#### Notifications

Send Telegram notifications on task completion:

```yaml
notifications:
- type: telegram
  api_token: env:TELEGRAM_BOT_TOKEN
  chat_id: env:TELEGRAM_CHAT_ID
  on_failure: true   # Send on any task failure
  on_success: false  # Don't send on success
```

Environment variables can be used with `env:VAR_NAME` syntax.

#### Agent Params

Pass CLI arguments to agents via `params` (or `args`):

```yaml
- agent: claude
  params: "--dangerously-skip-permissions"
  prompt: "Run ls"
  # Runs: claude -p --dangerously-skip-permissions "In <folder>: Run ls"

- agent: opencode
  params: "--model anthropic/claude-sonnet-4-6"
  prompt: "Review code"
  # Runs: opencode run --model anthropic/claude-sonnet-4-6 "In <folder>: Review code"

- agent: codex
  params: "--model o3"
  prompt: "Fix the bug"
  # Runs: codex exec --model o3 "In <folder>: Fix the bug"
```

Each agent maps to its non-interactive subcommand:

| Agent | Command format |
|-------|---------------|
| `claude` | `claude -p [params] "<prompt>"` |
| `opencode` | `opencode run [params] "<prompt>"` |
| `codex` | `codex exec [params] "<prompt>"` |

### Natural Language (`.htb`)

```natural-language
# Heartbeat: My check job
# Folder: ~/project
# Frequency: */15 * * * *
# Run once at: 2026-04-15 09:00

Every 15 minutes:
  - URL reachable: https://example.com
    On fail: echo "Site down!"
    
  - File exists: status.md
    On missing: echo "Status missing"
    
  - Ask Claude: Review code for bugs
    With: --resume mysession --dangerously-skip-permissions
```

## Frequency Shorthands

| Input | Cron |
|-------|------|
| `15min` | `*/15 * * * *` |
| `30min` | `*/30 * * * *` |
| `hourly` | `0 * * * *` |
| `daily` | `0 * * * *` |
| `daily 9` | `0 9 * * *` |
| `weekly` | `0 0 * * 0` |

## Scheduling

The heartbeat runs automatically when added to cron:

### Option 1: Cron (Recommended for Linux/macOS)

```bash
# Add job to crontab
heartbeat add-cron my-check

# View current cron entries
crontab -l

# Edit crontab manually
crontab -e

# Remove from cron
heartbeat remove my-check
```

Cron entry example:
```
*/15 * * * * python3 ~/.heartbeat/heartbeat.py -c '~/.heartbeat/jobs/my-check.yaml'
```

### Option 2: macOS Launchd

For macOS, you can use `launchd` instead of cron:

```bash
# Create a plist (for background check every 15 min)
cat > ~/Library/LaunchAgents/com.heartbeat.mycheck.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.heartbeat.mycheck</string>
    <key>ProgramArguments</key>
    <array>
        <string>python3</string>
        <string>/Users/maheshpatil/.heartbeat/heartbeat.py</string>
        <string>-c</string>
        <string>/Users/maheshpatil/.heartbeat/jobs/my-check.yaml</string>
    </array>
    <key>StartInterval</key>
    <integer>900</integer>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
EOF

# Load the agent
launchctl load ~/Library/LaunchAgents/com.heartbeat.mycheck.plist

# Unload
launchctl unload ~/Library/LaunchAgents/com.heartbeat.mycheck.plist
```

## Development

After making changes to the repo, deploy them to `~/.heartbeat`:

```bash
./deploy.sh
```

This copies `heartbeat.py`, the `heartbeat` CLI, any other shell scripts in the repo root, and all plugins to `~/.heartbeat/`, creating required directories if they don't exist.

## Testing

```bash
# Run tests
python3 -m pytest tests/ -v

# Run specific test
python3 -m pytest tests/test_heartbeat.py::test_parse_yaml -v

# Run with coverage
python3 -m pytest tests/ --cov=heartbeat --cov-report=term-missing
```

## File Structure

```
~/.heartbeat/
├── heartbeat.py       # Core script
├── heartbeat         # CLI wrapper
├── plugins/           # Notification plugins
│   ├── telegram.py   # Telegram notifications
│   ├── sms.py        # Stub (future)
│   ├── email.py      # Stub (future)
│   └── supabase.py  # Stub (future)
├── jobs/             # Job configs
│   ├── *.yaml
│   └── *.htb
└── logs/             # Log files
```

## Plugin System

Heartbeat supports extensible notification plugins. Place custom plugins in `~/.heartbeat/plugins/`.

### Built-in Plugins

| Plugin | Description |
|--------|-------------|
| telegram | Send messages via Telegram Bot API |
| sms | Stub (future) |
| email | Stub (future) |
| supabase | Stub (future) |

### Writing a Custom Plugin

```python
# ~/.heartbeat/plugins/my_plugin.py
from heartbeat.plugins.base import BasePlugin

class MyPlugin(BasePlugin):
    name = "my_plugin"
    
    def validate_config(self, config: dict) -> bool:
        return "token" in config
    
    def send(self, config: dict, message: str, **kwargs) -> bool:
        # Send notification logic here
        return True
```

Then use in your job config:

```yaml
notifications:
- type: my_plugin
  token: env:MY_PLUGIN_TOKEN
  on_failure: true
```

## License

MIT
