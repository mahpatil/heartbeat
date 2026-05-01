## MODIFIED Requirements

### Requirement: Automatic load after install
After writing the plist, `heartbeat install --autostart` SHALL call
`launchctl load ~/Library/LaunchAgents/com.heartbeat.plist` to start
the agent in the current session without requiring a logout. When the
LaunchAgent is not registered, `heartbeat daemon` SHALL print a startup
hint directing the user to run `heartbeat install --autostart`.

#### Scenario: Daemon starts immediately
- GIVEN `heartbeat install --autostart` completes
- WHEN `heartbeat list` is run immediately after
- THEN the daemon is reachable (socket exists)

#### Scenario: Startup hint shown when not registered
- GIVEN no LaunchAgent plist exists at `~/Library/LaunchAgents/com.heartbeat.plist`
- WHEN `heartbeat daemon` is run
- THEN it prints a tip: "run `heartbeat install --autostart` to start automatically at login"
- AND the daemon starts normally regardless
