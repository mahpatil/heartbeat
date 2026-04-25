<!-- heartbeat: schedule-job command -->
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
| **agent** | `claude` unless the user says opencode or codex |
| **workspace** | explicit path if mentioned, otherwise current directory (`$(pwd)`) |

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

```bash
heartbeat new \
  --name <name> \
  --schedule "<schedule>" \
  --prompt "<prompt>" \
  --agent <agent> \
  --workspace <workspace> \
  --flags=--dangerously-skip-permissions \
  --apply
```

Show the command before running it so the user can see what will be executed.

---

## Step 3 — Confirm

Run `heartbeat list` and show the output so the user can confirm the job is scheduled.

---

## Examples

```
/schedule-job review auth code every night at 2am in ~/projects/myapp
/schedule-job run /reviewer on this repo every hour
/schedule-job check https://myapp.com/health every 5 minutes
/schedule-job implement the spec in openspec/features/payments.md at 9am tomorrow
```
