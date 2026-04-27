## ADDED Requirements

### Requirement: Per-step timeout field
Shell and agent step definitions SHALL accept an optional `timeout` field
containing a human-readable duration string (e.g. `"10m"`, `"30s"`, `"1h"`).
When absent, the step runs without a time limit (existing behaviour).

#### Scenario: Timeout field parses successfully
- WHEN a job file contains `timeout: 10m` on a shell step
- THEN the step is loaded with a 600-second deadline

#### Scenario: Invalid timeout value is a load error
- WHEN a job file contains `timeout: banana` on a step
- THEN the daemon logs a parse error and skips the file

### Requirement: Step killed on timeout expiry
When a step's timeout elapses, the running child process SHALL be sent SIGKILL,
the step SHALL be marked failed with message `"step timed out after <N>s"`, and
the job's `on_fail` hooks SHALL execute.

#### Scenario: Shell step exceeds timeout
- GIVEN a shell step with `timeout: 5s` running `sleep 60`
- WHEN 5 seconds elapse
- THEN the `sleep` process is killed
- AND the step is marked failed
- AND `on_fail` hooks run

#### Scenario: Agent step exceeds timeout
- GIVEN an agent step with `timeout: 2m` whose subprocess does not exit
- WHEN 2 minutes elapse
- THEN the agent runner process is killed
- AND the step is marked failed

#### Scenario: Step completes before timeout
- GIVEN a shell step with `timeout: 60s` running `echo hello`
- WHEN the command exits in under 1 second
- THEN the step is marked successful
- AND no kill signal is sent
