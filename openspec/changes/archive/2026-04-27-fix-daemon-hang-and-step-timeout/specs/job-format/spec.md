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
