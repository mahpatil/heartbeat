## MODIFIED Requirements

### Requirement: Daemon-not-running error message
CLI commands that require IPC SHALL detect the absence of `heartbeat.sock`
and print a clear human-readable error before attempting to connect. Commands
that successfully connect but receive no response within 5 seconds SHALL also
print a clear human-readable error and exit non-zero.

#### Scenario: Socket does not exist
- GIVEN the daemon is not running
- WHEN `heartbeat list` is executed
- THEN it prints "heartbeat daemon is not running. Start it with: heartbeat daemon"
- AND exits with code 1

#### Scenario: Socket exists but daemon is unresponsive
- GIVEN the daemon process is alive but its runtime is hung
- WHEN `heartbeat list` is executed
- THEN it prints "Daemon not responding (timed out). It may be hung — restart with: heartbeat daemon"
- AND exits with code 1
