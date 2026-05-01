## ADDED Requirements

### Requirement: Wall-clock budget flag
The `algo-scout` binary SHALL accept a `--budget <secs>` flag (default: `900`,
i.e. 15 minutes) that limits total fetch time. When the budget elapses mid-loop
the binary SHALL stop fetching, emit a `PARTIAL RUN` marker, and exit 0 with
whatever results it has.

#### Scenario: Run completes within budget
- WHEN all 7 categories are fetched in under 900 seconds
- THEN the binary exits normally with full results and no PARTIAL RUN marker

#### Scenario: Budget expires mid-loop
- WHEN elapsed time exceeds the budget before all categories are fetched
- THEN the binary stops fetching immediately
- AND prints `algo-scout: PARTIAL RUN — budget exhausted after Ns (X/7 categories fetched)`
- AND exits 0 with partial results

#### Scenario: Budget overridden via flag
- WHEN `algo-scout --budget 120` is run interactively
- THEN the binary fetches all categories up to 120 seconds before stopping

### Requirement: Reduced curl timeout
The curl invocation SHALL use `--max-time 10` (reduced from 30). A category
fetch that does not complete within 10 seconds SHALL be skipped and counted
as 0 papers for that category.

#### Scenario: Slow category skipped cleanly
- WHEN a single arXiv category endpoint responds after 10 seconds
- THEN curl exits with timeout error
- AND the binary logs the failure and continues to the next category
- AND the run does not fail — it reports 0 papers for that category
