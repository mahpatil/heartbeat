# log-system Specification

## Purpose

Write per-job structured log output to files in `~/.heartbeat/logs/`,
stream output in real time (so `heartbeat logs` shows live LLM output),
and rotate files to prevent unbounded disk growth.

---

## Requirements

### Requirement: Per-job log files

Each job SHALL have its own log file at `~/.heartbeat/logs/<name>.log`.
The `logs/` directory SHALL be created by the daemon at startup if absent.

#### Scenario: Log file created on first run
- GIVEN no log file exists for `daily-review`
- WHEN the `daily-review` job fires for the first time
- THEN `~/.heartbeat/logs/daily-review.log` is created
- AND the first log line is written

---

### Requirement: Log line format

Each log line SHALL be formatted as:
```
<ISO-8601-timestamp> [<job-name>] <message>
```

Example:
```
2026-04-21T02:00:01Z [daily-review] --- starting ---
2026-04-21T02:00:02Z [daily-review] Reviewing git log...
2026-04-21T02:00:45Z [daily-review] --- completed in 44s ---
```

#### Scenario: Timestamp is UTC ISO-8601
- GIVEN a log line is written at 2026-04-21 02:00:01 UTC
- WHEN the line appears in the log file
- THEN it begins with `2026-04-21T02:00:01Z`

---

### Requirement: Real-time streaming

The executor SHALL write each line of agent/shell output to the log file
as it is received from the child process stdout/stderr, without buffering
entire output in memory first.

#### Scenario: Live output visible during execution
- GIVEN a claude agent step is producing streaming output
- WHEN `heartbeat logs daily-review` is running in another terminal
- THEN new lines appear in real time as the LLM streams tokens

---

### Requirement: Structured run boundaries

The logger SHALL write a start banner and a completion/failure line for
each job execution, making it easy to identify individual runs in the log.

```
2026-04-21T02:00:00Z [daily-review] ===== run started =====
...output lines...
2026-04-21T02:00:45Z [daily-review] ===== run completed (44s) =====
```

On failure:
```
2026-04-21T02:00:45Z [daily-review] ===== run FAILED: command exited 1 (44s) =====
```

---

### Requirement: Log rotation

When a log file reaches 10 MB, it SHALL be rotated. Up to 5 rotated files
SHALL be kept (`.log.1` through `.log.5`). The oldest (`.log.5`) is
overwritten when a new rotation occurs.

#### Scenario: Rotation at 10 MB
- GIVEN `daily-review.log` reaches 10 MB during an active run
- WHEN the next log line is written
- THEN `daily-review.log` is renamed to `daily-review.log.1`
- AND a new `daily-review.log` is opened
- AND writing continues without interruption

#### Scenario: Maximum 5 rotated files
- GIVEN 5 rotated files already exist (`.log.1` through `.log.5`)
- WHEN the next rotation occurs
- THEN `.log.5` is overwritten with the content of `.log.4`
- AND the chain shifts: `.log.4 ← .log.3`, `.log.3 ← .log.2`, etc.

---

### Requirement: Daemon-level tracing

The daemon itself SHALL use the `tracing` crate for structured logging.
Log level is controlled by `RUST_LOG` environment variable, defaulting to
`heartbeat=info`.

#### Scenario: Debug mode
- GIVEN `RUST_LOG=heartbeat=debug` is set
- WHEN `heartbeat daemon` starts
- THEN verbose debug lines including task scheduling decisions are printed

#### Scenario: Quiet mode
- GIVEN `RUST_LOG=heartbeat=warn`
- WHEN the daemon runs normally with no issues
- THEN nothing is printed to the terminal
