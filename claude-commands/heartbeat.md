<!-- heartbeat: /heartbeat command -->
Schedule a recurring heartbeat job from natural language: "$ARGUMENTS"

---

## Your task

Parse the user's request in `$ARGUMENTS` and create a heartbeat job using the non-interactive CLI. Do not ask unnecessary questions — infer reasonable defaults.

---

## Step 1 — Parse the request

From `$ARGUMENTS` extract:

| Field | How to infer |
|---|---|
| **name** | kebab-case slug from the subject (e.g. "review auth code" → `auth-code-review`) |
| **schedule** | time phrase → heartbeat syntax (see table below) |
| **prompt** | the action to perform, verbatim or cleaned up |
| **agent** | see agent inference table below |
| **workspace** | explicit path if mentioned, otherwise current directory (`$(pwd)`) |

**Agent inference:**

| User says | agent flag |
|---|---|
| "with claude", "ask claude", "claude", default | `claude` |
| "with opencode", "using opencode", "opencode" | `opencode` |
| "with codex", "using codex", "codex" | `codex` |

**Per-agent flags:**

| Agent | Always add |
|---|---|
| `claude` | `--flags=--dangerously-skip-permissions` |
| `opencode` | _(none by default — opencode runs headlessly)_ |
| `codex` | _(none by default)_ |

**Schedule inference:**

| User says | heartbeat syntax |
|---|---|
| "every hour", "hourly" | `every 1h` |
| "every N minutes" | `every Nm` |
| "every night", "nightly", "daily" | `daily at 02:00` |
| "every morning", "every day at 9" | `daily at 09:00` |
| "every N hours" | `every Nh` |
| "every week", "weekly" | `every 7d` |
| "once at HH:MM" | `once at HH:MM` |
| not specified | `every 1h` (default) |

---

## Step 2 — Run the command

Show the command before running it so the user can see what will be executed.

**Claude:**
```bash
heartbeat new \
  --name <name> \
  --schedule "<schedule>" \
  --prompt "<prompt>" \
  --agent claude \
  --workspace <workspace> \
  --flags=--dangerously-skip-permissions \
  --apply
```

**opencode:**
```bash
heartbeat new \
  --name <name> \
  --schedule "<schedule>" \
  --prompt "<prompt>" \
  --agent opencode \
  --workspace <workspace> \
  --apply
```

**codex:**
```bash
heartbeat new \
  --name <name> \
  --schedule "<schedule>" \
  --prompt "<prompt>" \
  --agent codex \
  --workspace <workspace> \
  --apply
```

---

## Step 3 — Confirm

Run `heartbeat list` and show the output so the user can confirm the job is scheduled.

---

## Examples

```
/heartbeat review auth code every night at 2am in ~/projects/myapp
/heartbeat run /reviewer on this repo every hour
/heartbeat check https://myapp.com/health every 5 minutes
/heartbeat implement the spec in openspec/features/payments.md with opencode every day at 9am
/heartbeat summarise git log daily using codex in ~/projects/backend
/heartbeat run /coder on openspec/issues/42.md once at 14:00
```
