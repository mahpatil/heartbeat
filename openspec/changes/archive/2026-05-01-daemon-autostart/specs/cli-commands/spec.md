## MODIFIED Requirements

### Requirement: `heartbeat list`
The daemon SHALL return a summary of all registered jobs. When the LaunchAgent
plist is not present, the list output SHALL append a one-line autostart hint
after the job table.

#### Scenario: No jobs registered
- GIVEN the jobs directory is empty
- WHEN `heartbeat list` is run
- THEN `data.jobs` is an empty array

#### Scenario: Autostart hint shown in list output
- GIVEN the daemon is running but no LaunchAgent plist exists
- WHEN `heartbeat list` is run
- THEN the job table is printed normally
- AND a hint line is printed below it: "Tip: daemon will not survive reboot — run `heartbeat install --autostart`"

#### Scenario: No hint when LaunchAgent is registered
- GIVEN `~/Library/LaunchAgents/com.heartbeat.plist` exists
- WHEN `heartbeat list` is run
- THEN no autostart hint is printed
