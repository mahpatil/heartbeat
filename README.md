# Heartbeat

A simple scheduled task runner with config as code. Runs tasks on a cron schedule, checks URLs, files, and can invoke AI agents (Claude, OpenCode, Codex).

## Features

- **Multiple formats**: YAML or natural language (.htb) config
- **Task types**: Run shell commands, check URLs, check file existence
- **Agent support**: Invoke Claude/OpenCode/Codex CLI or API
- **Easy cron setup**: Natural language frequency parsing
- **Zero deps**: Python 3 stdlib only

## Requirements

- Python 3.x (pre-installed on macOS/Linux)
- Optional: `claude`, `opencode`, or `codex` CLI for agent tasks
- Optional: `anthropic` or `openai` Python packages for API mode

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
heartbeat add "job name" --folder ~/project --frequency "every 15 min"
heartbeat add-cron "job name"    # Add to crontab
heartbeat list                 # List all jobs
heartbeat run "job name"       # Run manually
heartbeat logs [name]          # Show logs
heartbeat remove "job name"    # Remove job and cron
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
  prompt: "Review code for bugs"

- agent_api:
  provider: anthropic
  model: claude-sonnet-4-20250514
  prompt: "Summarize this file"
```

### Natural Language (`.htb`)

```natural-language
# Heartbeat: My check job
# Folder: ~/project

Every 15 minutes:
  - URL reachable: https://example.com
    On fail: echo "Site down!"
    
  - File exists: status.md
    On missing: echo "Status missing"
    
  - Ask Claude: Review code for bugs
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

## File Structure

```
~/.heartbeat/
├── heartbeat.py       # Core script
├── heartbeat         # CLI wrapper
├── jobs/             # Job configs
│   ├── *.yaml
│   └── *.htb
└── logs/             # Log files
```

## License

MIT