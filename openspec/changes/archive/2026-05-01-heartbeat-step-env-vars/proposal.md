## Why

There is no way to pass per-job environment variables to an agent invocation.
This matters for configuring agent-side settings (e.g. bash tool timeout,
model selection, API endpoint) on a per-job basis without touching global shell
config. The algo-scout job needs a higher bash tool timeout for opencode; other
jobs may need different API keys, endpoints, or feature flags.

## What Changes

- Job step definitions gain an optional `env:` map of string → string
- Environment variables in `env:` are injected into the child process before
  the agent runner (or shell command) is spawned
- All step types support `env:` — agent, shell, url-check, file-check
- The `heartbeat-agent-runner.sh` already inherits child env; no runner changes needed

Example usage:
```yaml
steps:
  - name: run
    type: agent
    agent: opencode
    prompt: "Run algo-scout skill"
    env:
      OPENCODE_BASH_TIMEOUT_MS: "120000"
      OPENCODE_MODEL: "anthropic/claude-sonnet-4-5"
```

## Capabilities

### New Capabilities
- `step-env-vars`: Per-step environment variable injection for all step types

### Modified Capabilities
- `job-format`: Step objects gain optional `env:` map field
- `agent-executor`: Executor applies `env:` vars to child process environment

## Impact

- `src/task/types.rs` — add `env: HashMap<String, String>` to all `StepDef` variants
- `src/job/config.rs` — parse `env:` map from YAML `RawStep`
- `src/task/executor.rs` — apply env vars via `.env()` on `tokio::process::Command`
  for both shell and agent steps
- `~/.heartbeat/jobs/algo-scout.htb` — add `env: { OPENCODE_BASH_TIMEOUT_MS: "120000" }`
- No new dependencies; `HashMap` is already in std
