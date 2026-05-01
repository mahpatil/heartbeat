## Context

The algo-scout binary fetches 7 arXiv categories sequentially. Each fetch
uses `curl --max-time 30` with a 3.1s inter-request delay (arXiv TOS minimum).
The binary is invoked via the bash tool inside opencode, which has a 60s
default timeout. When arXiv is slow, the binary is killed mid-run, producing
0 results. The fix lives entirely inside the binary — heartbeat has no role.

## Goals / Non-Goals

**Goals:**
- Binary always exits within the bash tool budget (default 50s)
- Partial results are surfaced clearly when the budget cuts a run short
- Faster failure on slow categories (lower curl max-time)

**Non-Goals:**
- Changing heartbeat job config or the agent runner
- Parallel HTTP fetching (would violate arXiv TOS ≥3s between requests)
- Retrying failed categories within the same run

## Decisions

### D1 — Wall-clock budget, not per-request count

Track `start_time` before the fetch loop. Before each category fetch, check
if `elapsed >= budget`. If so, break and emit partial results.

_Alternative: reduce per-request curl timeout only._ With `--max-time 10` and
7 calls: worst case = 7 × (10 + 3.1) = 91.7s — still exceeds 60s bash limit.
A budget cap is necessary regardless.

### D2 — Default budget: 50 seconds

Leaves 10s margin against the 60s bash tool timeout. Overridable via
`--budget <secs>` for local runs where there's no tool timeout constraint.

### D3 — Reduce curl max-time from 30s to 10s

Under normal arXiv conditions fetches complete in 2–5s. 10s provides enough
slack for a slow response without burning the budget on one stuck category.

### D4 — Emit `PARTIAL RUN` marker on early exit

When the budget expires mid-loop, the binary prints:
```
algo-scout: PARTIAL RUN — budget exhausted after Ns (X/7 categories fetched)
```
The AI in the skill reads this and reports the partial result honestly rather
than treating 0 candidates as "nothing found this month."

## Risks / Trade-offs

- **Budget too tight misses categories on a consistently slow day** → Mitigation:
  `--budget` override lets you run unconstrained locally; the monthly cadence
  means one missed category in a rare slow run is acceptable.
- **10s curl timeout drops a legitimate slow-but-present response** → Risk is
  low; arXiv CDN responses are consistently fast when available. 30s was
  over-engineered for a script expecting ~3s responses.

## Migration Plan

1. Edit `src/main.rs` in the algo-scout binary.
2. Rebuild: `bash skills/algo-scout/install.sh`.
3. No heartbeat restart needed; the binary is invoked fresh each run.
