# cli-commands Specification

## Purpose

Provide a single `heartbeat` binary with sub-commands for managing the daemon
and jobs. All sub-commands except `daemon` communicate with the running daemon
via the IPC socket.

---

## Requirements

### Requirement: `heartbeat daemon`

Start the heartbeat daemon in the foreground. Writes PID file, opens IPC
socket, loads jobs, blocks until shutdown signal.

```
heartbeat daemon
```

#### Scenario: Normal start
- GIVEN the daemon is not running
- WHEN `heartbeat daemon` is executed
- THEN the process stays in the foreground
- AND logs are written to stderr (or stdout) with timestamps

#### Scenario: Already running
- GIVEN `heartbeat.pid` exists and the PID is live
- WHEN `heartbeat daemon` is executed again
- THEN it prints "heartbeat is already running (PID N)" and exits 1

---

### Requirement: `heartbeat apply <file>`

Copy the given `.htb` file into `~/.heartbeat/jobs/` using the file's
basename. If the daemon is running, the filesystem watcher picks up the
change automatically. Does not require the daemon to be running.

```
heartbeat apply ~/my-jobs/daily-review.htb
```

#### Scenario: File copied successfully
- GIVEN `~/my-jobs/daily-review.htb` exists and is valid
- WHEN `heartbeat apply ~/my-jobs/daily-review.htb` is run
- THEN the file is copied to `~/.heartbeat/jobs/daily-review.htb`
- AND it prints "Applied: daily-review.htb"

#### Scenario: File does not exist
- GIVEN the path provided does not exist
- WHEN `heartbeat apply` is run
- THEN it prints "File not found: <path>" and exits 1

#### Scenario: Overwrite existing
- GIVEN `~/.heartbeat/jobs/daily-review.htb` already exists
- WHEN `heartbeat apply` is run with a new version
- THEN the file is overwritten (hot-reload triggers automatically if daemon is running)

---

### Requirement: `heartbeat list`

Print a table of all registered jobs with their name, status, schedule, and
next scheduled run time.

```
heartbeat list
```

Output format:
```
NAME             STATUS    SCHEDULE        NEXT RUN
daily-review     idle      daily at 02:00  2026-04-21 02:00 UTC
lint-check       running   every 30m       —
migrate-db       done      once at …       —
```

#### Scenario: No jobs
- GIVEN the daemon is running with an empty jobs directory
- WHEN `heartbeat list` is run
- THEN it prints "No jobs registered." and exits 0

---

### Requirement: `heartbeat run <name>`

Trigger an immediate out-of-schedule execution of the named job.

```
heartbeat run daily-review
```

#### Scenario: Successful trigger
- GIVEN `daily-review` is registered and idle
- WHEN `heartbeat run daily-review` is executed
- THEN it prints "Triggered: daily-review" and exits 0

#### Scenario: Job not found
- GIVEN no job named `foo` exists
- WHEN `heartbeat run foo` is executed
- THEN it prints "Error: job not found: foo" and exits 1

---

### Requirement: `heartbeat stop <name>`

Stop the scheduling loop for the named job without deleting its file.

```
heartbeat stop lint-check
```

#### Scenario: Job stopped
- GIVEN `lint-check` is running
- WHEN `heartbeat stop lint-check` is executed
- THEN it prints "Stopped: lint-check" and exits 0

---

### Requirement: `heartbeat logs <name>`

Tail the log file for the named job in real time. Equivalent to
`tail -F ~/.heartbeat/logs/<name>.log`. Does not require IPC — reads the file
directly. Exits on Ctrl-C.

```
heartbeat logs daily-review
heartbeat logs daily-review -n 50    # show last 50 lines before following
```

#### Scenario: Log file exists
- GIVEN `~/.heartbeat/logs/daily-review.log` exists
- WHEN `heartbeat logs daily-review` is run
- THEN it prints the last 100 lines and follows new output

#### Scenario: Log file does not exist
- GIVEN no log file exists for the named job
- WHEN `heartbeat logs missing-job` is run
- THEN it prints "No log file found for: missing-job" and exits 1

---

### Requirement: `heartbeat install`

One-time setup: creates `~/.heartbeat/` directory structure, copies the
binary and agent runner if not already present, and optionally configures
auto-start.

```
heartbeat install
heartbeat install --autostart      # also write LaunchAgent plist
```

#### Scenario: Fresh setup
- GIVEN `~/.heartbeat/` does not exist
- WHEN `heartbeat install` is run
- THEN directories are created
- AND a sample `.htb` file is written to `~/.heartbeat/jobs/`
- AND instructions are printed

#### Scenario: --autostart on macOS
- GIVEN `heartbeat install --autostart` is run on macOS
- THEN `~/Library/LaunchAgents/com.heartbeat.plist` is written
- AND `launchctl load` is called to start the agent
- AND it prints "heartbeat will start automatically at login"

---

### Requirement: Global `--help` and subcommand `--help`

All commands and subcommands SHALL support `--help` output describing
usage, arguments, and examples.

#### Scenario: Top-level help
- GIVEN no arguments
- WHEN `heartbeat --help` is run
- THEN it prints a summary of all subcommands and exits 0
