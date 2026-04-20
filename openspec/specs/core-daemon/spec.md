# core-daemon Specification

## Purpose

Provide a persistent, low-overhead Rust process that runs in the current
user's login session, loads job files from `~/.heartbeat/jobs/`, schedules
each job on an independent tokio task, and shuts down gracefully. The daemon
must never require root or any elevated privileges.

---

## Requirements

### Requirement: User-context execution

The daemon SHALL run as a regular user process, inheriting the full login
environment (`$HOME`, `$PATH`, macOS Keychain, `~/.config/` credential
stores) of the user who launched it.

#### Scenario: Agent CLIs can authenticate
- GIVEN the user has `ANTHROPIC_API_KEY` set in their shell environment
- WHEN the daemon is started with `heartbeat daemon`
- THEN the key is accessible to all child processes spawned by the daemon
- AND no additional credential configuration is required

#### Scenario: No sudo required
- GIVEN a standard macOS user account (non-admin)
- WHEN `heartbeat daemon` is executed
- THEN the daemon starts successfully without requesting elevated privileges

---

### Requirement: Environment loading

The daemon SHALL load `~/.heartbeat/.env` at startup if the file exists,
merging its variables into the process environment before any jobs run.

#### Scenario: .env loaded before first job
- GIVEN `~/.heartbeat/.env` contains `MY_TOKEN=abc123`
- WHEN the daemon starts and the first job fires
- THEN `$MY_TOKEN` is available to all tasks in that job

#### Scenario: Missing .env is not an error
- GIVEN `~/.heartbeat/.env` does not exist
- WHEN the daemon starts
- THEN it logs a debug message and continues normally

---

### Requirement: Job registry

The daemon SHALL maintain an in-memory registry of active jobs, keyed by
job name. Each entry tracks: the parsed `JobConfig`, the tokio `JoinHandle`,
the current `JobStatus`, and the job's `JobLogger`.

#### Scenario: Two jobs with the same name are rejected
- GIVEN two `.htb` files in the jobs directory resolve to the same `name:` field
- WHEN the daemon loads jobs at startup
- THEN only the first loaded is registered
- AND a warning is logged identifying the conflict

---

### Requirement: Jobs directory creation

The daemon SHALL create `~/.heartbeat/jobs/` and `~/.heartbeat/logs/` if
they do not exist at startup.

#### Scenario: Fresh install, no directories
- GIVEN `~/.heartbeat/jobs/` does not exist
- WHEN `heartbeat daemon` is run for the first time
- THEN the directory is created
- AND the daemon logs "Created jobs directory"

---

### Requirement: Graceful shutdown

The daemon SHALL handle `SIGTERM` and `SIGINT` by:
1. Stopping acceptance of new IPC connections.
2. Aborting all running job tasks.
3. Deleting `~/.heartbeat/heartbeat.pid` and `~/.heartbeat/heartbeat.sock`.
4. Flushing log buffers.
5. Exiting with code 0.

#### Scenario: Ctrl-C in terminal
- GIVEN the daemon is running in a terminal with active jobs
- WHEN the user presses Ctrl-C
- THEN all job tasks are cancelled within 2 seconds
- AND the PID file and socket file are removed
- AND the process exits with code 0

#### Scenario: SIGTERM from launchd
- GIVEN the daemon is running as a LaunchAgent
- WHEN macOS sends SIGTERM (e.g., at logout)
- THEN the same shutdown sequence completes cleanly

---

### Requirement: PID file

The daemon SHALL write its PID to `~/.heartbeat/heartbeat.pid` immediately
after binding the IPC socket. It SHALL delete this file on clean shutdown.
If a stale PID file exists at startup (process not running), the daemon SHALL
overwrite it with a warning log.

#### Scenario: Prevent double-start
- GIVEN `heartbeat.pid` exists and the process at that PID is alive
- WHEN a second `heartbeat daemon` is invoked
- THEN it prints "heartbeat is already running (PID N)" and exits with code 1

---

### Requirement: Hot-reload via filesystem events

The daemon SHALL watch `~/.heartbeat/jobs/` using the OS filesystem event
API (FSEvents on macOS via the `notify` crate). When a `.htb` file is
created, modified, or deleted, the daemon SHALL update the running job
registry without restarting.

#### Scenario: New job file dropped in jobs directory
- GIVEN the daemon is running with no jobs
- WHEN a valid `.htb` file is copied into `~/.heartbeat/jobs/`
- THEN the daemon detects the change within 500ms
- AND schedules the new job
- AND logs "Scheduling job: <name>"

#### Scenario: Job file deleted
- GIVEN a job named `daily-review` is running
- WHEN `~/.heartbeat/jobs/daily-review.htb` is deleted
- THEN the job task is cancelled
- AND the registry entry is removed

#### Scenario: Job file modified (reschedule)
- GIVEN a job is running with `schedule: every 1h`
- WHEN its `.htb` file is updated to `schedule: every 30m`
- THEN the old task is cancelled
- AND a new task starts with the updated schedule
