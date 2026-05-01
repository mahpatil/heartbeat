## Context

Heartbeat runs agent and shell steps as child processes via `execute_step`.
Child processes inherit the daemon's environment but there is no mechanism to
inject per-step variables. This is needed to configure per-job agent behaviour
(e.g. opencode bash tool timeout, model selection) without touching the global
shell environment.

## Goals / Non-Goals

**Goals:**
- Any step type (agent, shell, url-check, file-check) can specify `env:` key-
  value pairs in YAML
- Variables are injected as process environment vars before the child spawns
- Existing behaviour is unchanged when `env:` is absent

**Non-Goals:**
- Secret management (no vault integration — env vars are plaintext in the job file)
- Variable interpolation (e.g. `${HOME}/path`) — use the shell step for that
- Inheriting or overriding daemon-level env vars beyond what's specified

## Decisions

### D1 — `env: HashMap<String, String>` on all StepDef variants

Add `env` to Shell and Agent (the two variants that actually spawn processes).
UrlCheck and FileCheck don't spawn external processes so env has no effect
there — include the field in the YAML parser for forward-compat but don't
apply it in the executor.

_Alternative: only on Agent steps._ Rejected — shell steps also benefit (e.g.
setting `GITHUB_TOKEN` for a specific shell step).

### D2 — Apply env via `.env()` chain on `tokio::process::Command`

The env map is iterated and each pair added with `.env(key, value)`. This
merges with the inherited environment (daemon env + step env). Variables in
the step's `env:` override any daemon-level value with the same name.

### D3 — YAML `env:` is a flat string→string map

```yaml
env:
  KEY: value
  ANOTHER_KEY: "value with spaces"
```

serde_yaml deserialises this directly into `HashMap<String, String>`. No
special parsing needed.

## Risks / Trade-offs

- **Secrets in plaintext job files** → acceptable for local daemon on personal
  machine; document that sensitive values should use `.env` file instead
- **env override silently wins** → expected behaviour; document clearly

## Migration Plan

1. Add `env` field to `StepDef` and `RawStep`.
2. Wire through `executor.rs`.
3. Update `algo-scout.htb` with the opencode bash timeout var.
4. No daemon restart needed for new job files (hot-reload picks them up).
