## Why

The algo-scout binary fetches 7 arXiv categories sequentially with a 30s curl
timeout and a 3.1s inter-request delay. When arXiv is slow, the total runtime
crosses the 60s bash tool timeout inside opencode, causing the process to be
killed mid-run and returning 0 candidates. The binary needs an internal wall-
clock budget so it stops gracefully before the bash tool kills it.

## What Changes

- `algo-scout` binary gains a `--budget <secs>` flag (default: `50`); when the
  budget elapses the binary prints whatever results it has so far and exits cleanly
- `curl --max-time` is reduced from `30` to `10` — fast enough for arXiv under
  normal conditions, slow enough to not miss papers on a busy API day
- A `PARTIAL RUN` warning line is appended to output when the budget cuts the
  run short, so the AI knows results may be incomplete and can suggest a retry

## Capabilities

### New Capabilities
- `algo-scout-budget`: Wall-clock budget limiting the binary's total fetch time,
  with graceful partial-result output when the budget expires

### Modified Capabilities

_(none — this change is entirely within the algo-scout binary, not heartbeat)_

## Impact

- `skills/algo-scout/algo-scout/src/main.rs` — add `--budget` CLI arg, track
  elapsed time in the fetch loop, break early when budget is exceeded
- `skills/algo-scout/algo-scout/src/main.rs` — change curl `--max-time` from
  `30` to `10`
- `skills/algo-scout/install.sh` — rebuild step picks up the change automatically
- No heartbeat changes; no new dependencies
