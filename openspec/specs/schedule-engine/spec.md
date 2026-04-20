# schedule-engine Specification

## Purpose

Parse the `schedule:` field in job files into a strongly-typed `Schedule`
enum and drive tokio timers accordingly. The format is intentionally
human-readable — no cron syntax required.

---

## Requirements

### Requirement: Interval schedule (`every N<unit>`)

The engine SHALL parse `every <N><unit>` into a repeating interval timer.
Supported units: `s`/`sec`/`secs`, `m`/`min`/`mins`, `h`/`hr`/`hrs`,
`d`/`day`/`days`.

The first execution SHALL fire immediately when the job is registered
(no initial delay). Subsequent executions start after each interval
completes (missed-tick behavior: `Delay`, not `Burst`).

#### Scenario: every 5m
- GIVEN `schedule: every 5m`
- WHEN the job is registered at 10:00:00
- THEN it fires at 10:00:00, 10:05:00, 10:10:00, …

#### Scenario: Overrun does not cause burst
- GIVEN `schedule: every 5m` and the task takes 6 minutes
- WHEN the task completes at 10:06:00
- THEN the next fire time is 10:11:00 (not 10:05:00 backlog)

#### Scenario: Invalid unit
- GIVEN `schedule: every 5x`
- WHEN the job file is loaded
- THEN parsing fails with "unknown time unit 'x'"
- AND the job is skipped

---

### Requirement: Daily schedule (`daily at HH:MM`)

The engine SHALL parse `daily at HH:MM` (24-hour) and sleep until the
next occurrence of that wall-clock time in the local timezone.

After a job fires, the engine SHALL sleep until the same time the next
calendar day (adding 61 seconds before computing the next target to
avoid same-minute re-fire).

#### Scenario: Future time today
- GIVEN `schedule: daily at 14:30` and current time is 09:00
- WHEN the job is registered
- THEN the first execution is at 14:30 today

#### Scenario: Past time today (schedule tomorrow)
- GIVEN `schedule: daily at 02:00` and current time is 09:00
- WHEN the job is registered
- THEN the first execution is at 02:00 tomorrow

#### Scenario: Invalid time format
- GIVEN `schedule: daily at 25:00`
- WHEN the job file is loaded
- THEN parsing fails with "invalid time: 25:00"

---

### Requirement: One-shot schedule (`once at <datetime>`)

The engine SHALL parse `once at <value>` where `<value>` is either:
- An RFC-3339 datetime string: `2026-04-21T03:00:00`
- A bare `HH:MM` (resolves to the next occurrence of that time today/tomorrow)

After the task fires, the job task exits. The controller removes the
registry entry and logs "Job completed (once-at): <name>".

#### Scenario: Future RFC-3339 datetime
- GIVEN `schedule: once at 2026-04-21T03:00:00`
- WHEN the job is registered at 2026-04-20T09:00:00
- THEN the task fires once at 2026-04-21T03:00:00 UTC
- AND the job is removed from the registry afterward

#### Scenario: Past datetime fires immediately
- GIVEN `schedule: once at 2020-01-01T00:00:00`
- WHEN the job is registered
- THEN the task fires immediately (target is in the past)
- AND a warning is logged: "schedule is in the past, running immediately"

#### Scenario: Bare HH:MM
- GIVEN `schedule: once at 14:00` and current time is 09:00
- WHEN the job is registered
- THEN it fires at 14:00 today

---

### Requirement: Schedule parse error reporting

When `Schedule::parse()` fails, the engine SHALL return a descriptive
`anyhow::Error` that includes the original string and the reason.

#### Scenario: Completely unrecognised string
- GIVEN `schedule: "run it sometime"`
- WHEN the job file is loaded
- THEN parsing fails with `unrecognised schedule: "run it sometime"`
- AND the job is skipped with a warning log
