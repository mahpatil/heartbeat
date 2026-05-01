## MODIFIED Requirements

### Requirement: Optional fields
The following frontmatter fields are optional:

| Field | Type | Default | Description |
|---|---|---|---|
| `workspace` | string | `~` | Working directory for tasks |
| `agent` | string | `claude` | Default agent when no `steps:` array |
| `flags` | string[] | `[]` | Extra CLI flags for the agent |
| `on_fail` | string[] | `[]` | Shell commands to run on any task failure |
| `steps` | object[] | — | Explicit multi-step pipeline (see chained-steps spec) |

Step objects within `steps:` accept the following optional fields:

| Field | Type | Default | Description |
|---|---|---|---|
| `timeout` | string | — | Kill the step after this duration (e.g. `"10m"`, `"30s"`). Absent means no limit. |
| `env` | map[string]string | `{}` | Environment variables injected into the child process before it spawns. |

#### Scenario: workspace defaults to home directory
- GIVEN a job file with no `workspace:` field
- WHEN a task runs
- THEN the working directory is `$HOME`

#### Scenario: env map is parsed from YAML
- GIVEN a step with `env: { FOO: bar, NUM: "42" }`
- WHEN the job file is loaded
- THEN the step has env map `{ "FOO": "bar", "NUM": "42" }`

#### Scenario: env absent means empty map
- GIVEN a step with no `env:` key
- WHEN the job file is loaded
- THEN the step has an empty env map and child process inherits daemon environment unchanged
