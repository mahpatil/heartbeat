## ADDED Requirements

### Requirement: Per-step environment variable injection
Agent and shell step definitions SHALL accept an optional `env:` map of
string key-value pairs. Before the child process is spawned, each pair SHALL
be added to the process environment. Variables in `env:` override any
same-named variable inherited from the daemon environment.

#### Scenario: env var reaches child process
- GIVEN a shell step with `env: { MY_VAR: "hello" }`
- WHEN the step runs `echo $MY_VAR`
- THEN the output logged is `hello`

#### Scenario: env var overrides daemon environment
- GIVEN the daemon was started with `FOO=bar` in its environment
- AND a shell step has `env: { FOO: "override" }`
- WHEN the step runs `echo $FOO`
- THEN the output is `override`

#### Scenario: absent env field has no effect
- GIVEN a step with no `env:` field
- WHEN the step executes
- THEN the child process inherits the daemon environment unchanged

#### Scenario: agent step receives env var
- GIVEN an agent step with `env: { OPENCODE_BASH_TIMEOUT_MS: "120000" }`
- WHEN the agent runner is spawned
- THEN `OPENCODE_BASH_TIMEOUT_MS=120000` is present in the runner's environment
