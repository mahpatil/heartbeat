# chained-steps Specification

## Purpose

Allow a single job file to define an ordered pipeline of steps — mixing
agent calls (claude, opencode, codex), shell commands, and checks. Steps
run sequentially. A failure in any step halts the pipeline and triggers
`on_fail`.

---

## Requirements

### Requirement: Steps array

When the frontmatter contains a `steps:` array, each element defines one
step. The prompt body is ignored.

```yaml
---
name: nightly-pipeline
schedule: daily at 01:30
workspace: ~/projects/myapp
on_fail:
  - notify-slack.sh "nightly-pipeline failed"
steps:
  - name: run-tests
    type: shell
    command: cargo test 2>&1 | tee /tmp/test-output.txt

  - name: summarise
    type: agent
    agent: claude
    prompt: |
      Read /tmp/test-output.txt and summarise any failures.
      Write the summary to /tmp/test-summary.md.

  - name: health-check
    type: url-check
    url: https://myapp.example.com/health
---
```

#### Scenario: All steps succeed
- GIVEN a job with 3 steps that all complete without error
- WHEN the job fires
- THEN all 3 steps run in order
- AND the job is marked completed

#### Scenario: Step 2 fails, steps 3+ are skipped
- GIVEN a 3-step job where step 2 exits non-zero
- WHEN step 2 fails
- THEN step 3 is not executed
- AND `on_fail` commands run
- AND the job is marked failed

---

### Requirement: Step types

The `type:` field in each step SHALL accept: `agent`, `shell`, `url-check`,
`file-check`. The `type:` field is required for each step.

#### Scenario: Unknown step type
- GIVEN a step with `type: banana`
- WHEN the job file is parsed
- THEN parsing fails with "unknown step type: banana"
- AND the job is skipped

---

### Requirement: Step-level workspace override

Each step MAY include its own `workspace:` field that overrides the
job-level workspace for that step only.

```yaml
steps:
  - name: backend-tests
    type: shell
    command: cargo test
    workspace: ~/projects/myapp/backend

  - name: frontend-tests
    type: shell
    command: npm test
    workspace: ~/projects/myapp/frontend
```

#### Scenario: Step workspace overrides job workspace
- GIVEN a job with `workspace: ~/projects/myapp` and a step with `workspace: ~/projects/other`
- WHEN that step executes
- THEN the working directory is `~/projects/other`

---

### Requirement: Step-level agent override

Each agent step MAY specify its own `agent:` and `flags:` fields,
overriding the job-level defaults.

```yaml
---
name: multi-agent
schedule: every 1h
agent: claude          # default
steps:
  - name: code-review
    type: agent
    prompt: Review the last commit for issues.

  - name: opencode-step
    type: agent
    agent: opencode    # override
    prompt: Fix the issues found in the review.
---
```

#### Scenario: Step agent overrides job agent
- GIVEN a job with `agent: claude` and a step with `agent: opencode`
- WHEN that step executes
- THEN `opencode` is invoked, not `claude`

---

### Requirement: Step names are optional but logged

Step `name:` fields are optional. If present, they are used in log output
for easier reading. If absent, the step is logged by its index (e.g.,
`step[1]`).

#### Scenario: Named step in logs
- GIVEN a step with `name: run-tests`
- WHEN that step starts
- THEN the log reads "2026-04-21T01:30:00Z [nightly-pipeline] [run-tests] starting..."

#### Scenario: Unnamed step in logs
- GIVEN a step with no `name:` field at index 2
- WHEN that step starts
- THEN the log reads "2026-04-21T01:30:00Z [nightly-pipeline] [step[2]] starting..."

---

### Requirement: Single-step shorthand remains valid

A job file with no `steps:` array and a prompt body SHALL continue to work
as a single-step job (see job-format spec). The `chained-steps` feature is
purely additive.
