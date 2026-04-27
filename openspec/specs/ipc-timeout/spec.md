# ipc-timeout Specification

## Purpose

Ensure CLI commands that communicate with the daemon via the Unix socket do not
hang indefinitely if the daemon process is alive but unresponsive. A hard
deadline gives users a clear, actionable error instead of a frozen terminal.

---

## Requirements

### Requirement: CLI IPC deadline

All CLI commands that communicate with the daemon via `send_ipc` SHALL apply a
hard 5-second deadline. If the daemon does not respond within that window the
command SHALL print a human-readable error and exit non-zero.

#### Scenario: Daemon responds within deadline
- WHEN `heartbeat list` is run and the daemon replies within 5 s
- THEN the normal output is printed
- AND the process exits 0

#### Scenario: Daemon does not respond within deadline
- WHEN `heartbeat list` is run and the daemon fails to respond within 5 s
- THEN the CLI prints "Daemon not responding (timed out). It may be hung — restart with: heartbeat daemon"
- AND exits with code 1

#### Scenario: Deadline applies to all subcommands
- WHEN any of `list`, `run`, `stop`, `logs`, `ping` are executed
- THEN each applies the same 5-second IPC deadline
