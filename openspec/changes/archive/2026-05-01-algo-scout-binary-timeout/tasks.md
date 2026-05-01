## Tasks

- [x] Add `--budget <secs>` CLI argument to `algo-scout` binary (`~/agent-os/skills/algo-scout/algo-scout/src/main.rs`) with default `50`; parse with `clap` or manual arg matching (already uses manual argv parsing)
- [x] Record `start_time = Instant::now()` before the fetch loop; break out of the category loop when `start_time.elapsed().as_secs() >= budget`, printing a `PARTIAL RUN (budget exhausted after Xs — N of 7 categories fetched)` warning line before exit
- [x] Reduce `curl --max-time` from `30` to `10` in the `curl()` helper function
- [x] Rebuild the binary: run `cargo build --release` inside `~/agent-os/skills/algo-scout/algo-scout/` and copy the binary to the installed location (check `install.sh` for the target path)
- [x] Manually verify: run `algo-scout --budget 5` and confirm the PARTIAL RUN line appears after roughly 5s
