# job-format Specification

## Purpose

Define the `.htb` file format used to declare jobs. A job file is a single
Markdown-style document with YAML frontmatter (machine-readable config) and a
free-text prompt body (human-readable instruction). This keeps scheduling
metadata separate from the natural-language prompts sent to agents.

---

## Requirements

### Requirement: Frontmatter delimiter

A job file SHALL begin with `---` on the first line, contain YAML key-value
pairs, and close with `---` on its own line. Everything after the closing
delimiter is the prompt body.

#### Scenario: Valid file is parsed
- GIVEN a file with `---` open/close delimiters and valid YAML inside
- WHEN the daemon loads the file
- THEN frontmatter fields are parsed into a `JobConfig` struct
- AND the text after `---` is stored as the prompt body string

#### Scenario: Missing closing delimiter is an error
- GIVEN a file that starts with `---` but never closes it
- WHEN the daemon attempts to load the file
- THEN it logs a parse error for that file
- AND skips it without crashing

---

### Requirement: Required fields

Every job file SHALL provide `name` and `schedule` in the frontmatter.

#### Scenario: Missing name field
- GIVEN a `.htb` file with no `name:` key
- WHEN the daemon loads the file
- THEN it logs "missing required field: name" for that file
- AND skips it

#### Scenario: Missing schedule field
- GIVEN a `.htb` file with `name:` but no `schedule:` key
- WHEN the daemon loads the file
- THEN it logs "missing required field: schedule" for that file
- AND skips it

---

### Requirement: Optional fields

The following frontmatter fields are optional:

| Field | Type | Default | Description |
|---|---|---|---|
| `workspace` | string | `~` | Working directory for tasks |
| `agent` | string | `claude` | Default agent when no `steps:` array |
| `flags` | string[] | `[]` | Extra CLI flags for the agent |
| `on_fail` | string[] | `[]` | Shell commands to run on any task failure |
| `steps` | object[] | — | Explicit multi-step pipeline (see chained-steps spec) |

Step objects within `steps:` accept the following optional field:

| Field | Type | Default | Description |
|---|---|---|---|
| `timeout` | string | — | Kill the step after this duration (e.g. `"10m"`, `"30s"`). Absent means no limit. |

#### Scenario: workspace defaults to home directory
- GIVEN a job file with no `workspace:` field
- WHEN a task runs
- THEN the working directory is `$HOME`

#### Scenario: Step timeout specified in minutes
- GIVEN a shell step with `timeout: 10m`
- WHEN the step is loaded
- THEN its deadline is set to 600 seconds

---

### Requirement: Body-as-prompt (single-agent shorthand)

When `steps:` is absent, the prompt body (text after the closing `---`) SHALL
be used as the prompt for a single agent step using the `agent:` and `flags:`
frontmatter fields.

```
---
name: daily-review
schedule: daily at 02:00
workspace: ~/projects/myapp
agent: claude
flags: [--model claude-opus-4-5]
---
Review git log from the last 24 hours.
Flag any commits touching auth or payments.
```

#### Scenario: Body becomes single step
- GIVEN a file with no `steps:` and a non-empty body
- WHEN the daemon loads the file
- THEN it generates one `AgentStep` with `prompt = body.trim()`

#### Scenario: Empty body with no steps is an error
- GIVEN a file with no `steps:` and an empty body
- WHEN the daemon loads the file
- THEN it logs "job has no steps and no prompt body" and skips it

---

### Requirement: File extension

The daemon SHALL only load files with the `.htb` extension from the jobs
directory. All other files (including `.yaml`, `.md`, `.txt`) SHALL be ignored
silently.

#### Scenario: Non-.htb file is ignored
- GIVEN `~/.heartbeat/jobs/notes.txt` exists
- WHEN the daemon scans the jobs directory
- THEN `notes.txt` is not loaded
- AND no warning is logged for it

---

### Requirement: Job name uniqueness via file stem

The job `name` field in frontmatter SHALL be used as the registry key. If
two files have the same `name` value, the second is rejected with a warning.
As a convention, `name` SHOULD match the file stem (e.g., `daily-review.htb`
→ `name: daily-review`) but this is not enforced.

---

### Requirement: Shell comments in frontmatter

Lines beginning with `#` inside the frontmatter block SHALL be treated as
YAML comments and ignored by the parser.

#### Scenario: Comments in frontmatter are valid
- GIVEN a file with `# This runs every night` inside the `---` block
- WHEN the file is parsed
- THEN the comment is ignored and no error is raised
