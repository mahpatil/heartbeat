# ipc-protocol Specification

## Purpose

Allow the `heartbeat` CLI (and any future tooling) to communicate with the
running daemon without restarting it. Uses a Unix domain socket for
zero-configuration, user-space IPC that works without network access or
elevated privileges.

---

## Requirements

### Requirement: Socket location

The daemon SHALL create a Unix domain socket at `~/.heartbeat/heartbeat.sock`
immediately after startup and remove it on clean shutdown.

#### Scenario: Socket created on daemon start
- GIVEN the daemon is not running
- WHEN `heartbeat daemon` is executed
- THEN `~/.heartbeat/heartbeat.sock` exists within 500ms

#### Scenario: Stale socket cleaned up
- GIVEN a stale `heartbeat.sock` from a crashed daemon exists
- WHEN `heartbeat daemon` starts
- THEN it removes the stale socket, binds a new one, and continues

---

### Requirement: Wire format

All messages SHALL be UTF-8 JSON objects terminated by a single newline (`\n`).
Each client connection sends exactly one request and receives exactly one
response, then closes.

Request schema:
```json
{ "id": "<uuid-v4>", "cmd": "<command>", "name": "<job-name>" }
```
`name` is optional and only required for commands that target a specific job.

Response schema:
```json
{ "id": "<uuid-v4>", "ok": true, "data": { ... } }
{ "id": "<uuid-v4>", "ok": false, "error": "<message>" }
```

#### Scenario: Request ID echoed in response
- GIVEN a request with `"id": "abc-123"`
- WHEN the daemon responds
- THEN the response contains `"id": "abc-123"`

---

### Requirement: `ping` command

The daemon SHALL respond to `ping` with `{ "ok": true, "data": { "pong": true } }`.
Used by CLI commands to verify the daemon is alive before sending real requests.

#### Scenario: Daemon alive
- GIVEN the daemon is running
- WHEN `{ "cmd": "ping" }` is sent
- THEN `{ "ok": true, "data": { "pong": true } }` is received

---

### Requirement: `list` command

The daemon SHALL return a summary of all registered jobs.

Response `data` schema:
```json
{
  "jobs": [
    {
      "name": "daily-review",
      "status": "idle",
      "schedule": "daily at 02:00",
      "workspace": "~/projects/myapp",
      "next_run": "2026-04-21T02:00:00Z"
    }
  ]
}
```

Status values: `idle`, `running`, `failed`, `done`.

#### Scenario: No jobs registered
- GIVEN the jobs directory is empty
- WHEN `list` is sent
- THEN `data.jobs` is an empty array

---

### Requirement: `run` command

The daemon SHALL trigger an immediate out-of-schedule execution of the named
job. The job's interval timer is not reset; it continues on its normal cadence.

#### Scenario: Immediate trigger
- GIVEN `daily-review` is registered and currently idle
- WHEN `{ "cmd": "run", "name": "daily-review" }` is sent
- THEN the job starts executing immediately
- AND `{ "ok": true }` is returned

#### Scenario: Job not found
- GIVEN no job named `missing-job` is registered
- WHEN `{ "cmd": "run", "name": "missing-job" }` is sent
- THEN `{ "ok": false, "error": "job not found: missing-job" }` is returned

#### Scenario: Already running
- GIVEN `daily-review` is currently running
- WHEN `{ "cmd": "run", "name": "daily-review" }` is sent
- THEN `{ "ok": false, "error": "job is already running" }` is returned

---

### Requirement: `stop` command

The daemon SHALL cancel the scheduling loop for the named job without
removing its file from the jobs directory. The job can be restarted via
`run` or a reload.

#### Scenario: Stop a running job
- GIVEN `lint-check` is registered and running
- WHEN `{ "cmd": "stop", "name": "lint-check" }` is sent
- THEN the job task is cancelled
- AND `{ "ok": true }` is returned
- AND the job status becomes `idle` (stopped)

---

### Requirement: `reload` command

The daemon SHALL re-scan the jobs directory, applying any file additions,
modifications, or removals — the same logic triggered by filesystem events.

#### Scenario: Force reload
- GIVEN a new `.htb` file was added but the watcher hasn't fired yet
- WHEN `{ "cmd": "reload" }` is sent
- THEN the new job is picked up immediately
- AND `{ "ok": true }` is returned

---

### Requirement: Daemon-not-running error message

CLI commands that require IPC SHALL detect the absence of `heartbeat.sock`
and print a clear human-readable error before attempting to connect.

#### Scenario: Socket does not exist
- GIVEN the daemon is not running
- WHEN `heartbeat list` is executed
- THEN it prints "heartbeat daemon is not running. Start it with: heartbeat daemon"
- AND exits with code 1
