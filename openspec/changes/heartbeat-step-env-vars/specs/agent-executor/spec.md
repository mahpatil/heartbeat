## MODIFIED Requirements

### Requirement: User context inheritance
All child processes spawned by the executor SHALL inherit the full environment
of the daemon process (which itself inherits the user's login environment).
When a step defines an `env:` map, those key-value pairs SHALL be merged on
top of the inherited environment before the child process is spawned. Step-
level variables take precedence over daemon-level variables with the same name.

#### Scenario: Claude CLI can authenticate
- GIVEN `ANTHROPIC_API_KEY` is set in the user's shell environment
- WHEN an agent step with `agent: claude` fires
- THEN `claude` authenticates successfully without additional config
- AND the task completes without an auth error

#### Scenario: Homebrew binary is found
- GIVEN `/opt/homebrew/bin` is not in the system `$PATH` but is in the user's PATH
- WHEN a shell step runs `brew list`
- THEN the command succeeds because the daemon inherited the user's PATH

#### Scenario: Step env overrides daemon env
- GIVEN `MODEL=claude-haiku` in the daemon environment
- AND a step defines `env: { MODEL: claude-opus-4-5 }`
- WHEN the step's child process reads `MODEL`
- THEN it sees `claude-opus-4-5`
