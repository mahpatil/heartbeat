# agent-executor Specification

## Purpose

Execute individual job steps — agent calls (claude, opencode, codex) and
shell commands — in the user's login context. Agent steps are routed through
`heartbeat-agent-runner.sh`, which handles PATH enrichment and CLI dispatch.
Output is streamed in real time to the job's log file.

---

## Requirements

### Requirement: User context inheritance

All child processes spawned by the executor SHALL inherit the full environment
of the daemon process (which itself inherits the user's login environment).

#### Scenario: Claude CLI can authenticate
- GIVEN `ANTHROPIC_API_KEY` is set in the user's shell environment
- WHEN an agent step with `agent: claude` fires
- THEN `claude` authenticates successfully without additional config
- AND the task completes without an auth error

#### Scenario: Homebrew binary is found
- GIVEN `/opt/homebrew/bin` is not in the system `$PATH` but is in the user's PATH
- WHEN a shell step runs `brew list`
- THEN the command succeeds because the daemon inherited the user's PATH

---

### Requirement: Agent runner discovery

The executor SHALL locate `heartbeat-agent-runner.sh` by searching in order:
1. The directory containing the `heartbeat` binary.
2. `~/.heartbeat/heartbeat-agent-runner.sh`.
3. `$PATH` (via `which`).

If not found in any location, the step SHALL fail with a descriptive error.

#### Scenario: Runner found next to binary
- GIVEN the binary is at `~/.heartbeat/heartbeat` and runner is at `~/.heartbeat/heartbeat-agent-runner.sh`
- WHEN an agent step is about to execute
- THEN the runner at `~/.heartbeat/heartbeat-agent-runner.sh` is used

#### Scenario: Runner not found anywhere
- GIVEN `heartbeat-agent-runner.sh` is not in any search location
- WHEN an agent step fires
- THEN the step fails with "heartbeat-agent-runner.sh not found in PATH or ~/.heartbeat/"
- AND the on_fail commands run

---

### Requirement: Agent step invocation

An agent step SHALL invoke the runner as:

```
heartbeat-agent-runner.sh -l <log_path> <agent> <workspace> <prompt> [flags...]
```

Supported agent values: `claude`, `opencode`, `codex`, `shell`.

#### Scenario: Claude agent step
- GIVEN a step with `agent: claude`, `prompt: "review changes"`, `flags: [--model claude-opus-4-5]`
- WHEN the step executes
- THEN the runner is called with: `-l /path/to/job.log claude ~/workspace "review changes" --model claude-opus-4-5`

#### Scenario: Unknown agent value passed through
- GIVEN a step with `agent: my-custom-cli`
- WHEN the step executes
- THEN the runner calls `my-custom-cli <prompt>` as a fallback (runner's `*` case)

---

### Requirement: Real-time output streaming

The executor SHALL pipe stdout and stderr of the runner process to the
`JobLogger` line by line as they are produced, not after the process exits.

#### Scenario: LLM output appears in log immediately
- GIVEN a claude agent step that produces streaming output
- WHEN `heartbeat logs <name>` is running in another terminal
- THEN log lines appear as the LLM generates them, not only at task completion

---

### Requirement: Shell step execution

Shell steps (`type: shell`) SHALL be executed directly via `bash -c <command>`
without going through the agent runner. The same streaming pipe pattern
applies.

#### Scenario: Shell step runs in workspace
- GIVEN a shell step with `command: cargo test` and `workspace: ~/projects/myapp`
- WHEN the step fires
- THEN `bash -c "cargo test"` runs with cwd set to `~/projects/myapp`

#### Scenario: Non-zero exit code is a failure
- GIVEN a shell step whose command exits with code 1
- WHEN the command completes
- THEN the step is marked failed
- AND `on_fail` commands execute

---

### Requirement: on_fail execution

When any step in a job fails, the `on_fail` list from the job frontmatter
SHALL be executed as sequential shell commands. Failures in `on_fail`
commands are logged but do not trigger further `on_fail` recursion.

#### Scenario: on_fail runs after step failure
- GIVEN `on_fail: ["notify-slack.sh 'job failed'"]`
- WHEN a step exits non-zero
- THEN `notify-slack.sh 'job failed'` is run
- AND subsequent steps in the job are skipped

---

### Requirement: URL-check step

A `url-check` step SHALL issue an HTTP HEAD (or GET) request to the given
URL and verify the response status matches `expected_status` (default: 200).
This step does not use the agent runner.

#### Scenario: Healthy endpoint
- GIVEN `url: https://example.com/health` and `expected_status: 200`
- WHEN the step fires
- THEN a GET request is made and status 200 is received
- AND the step succeeds

#### Scenario: Unexpected status fails the step
- GIVEN `expected_status: 200` and the server returns 503
- WHEN the step fires
- THEN the step fails with "url returned 503 (expected 200)"

---

### Requirement: File-check step

A `file-check` step SHALL verify that the given path exists on the
filesystem. Tilde expansion SHALL be applied.

#### Scenario: File exists
- GIVEN `path: ~/projects/myapp/Cargo.toml` and the file is present
- WHEN the step fires
- THEN the step succeeds

#### Scenario: File missing fails the step
- GIVEN `path: ~/missing-file.txt`
- WHEN the step fires
- THEN the step fails with "file not found: /Users/…/missing-file.txt"
