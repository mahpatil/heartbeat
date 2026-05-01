## Tasks

- [x] In `src/cli/daemon_cmd.rs`, after the daemon starts its event loop, check if the LaunchAgent plist exists via `plist_path()` (already used in `src/cli/install.rs`); if absent, print: `tip: run \`heartbeat install --autostart\` to start automatically at login`
- [x] In `src/cli/list.rs`, after printing the job table, check if the LaunchAgent plist exists; if absent, append a blank line then: `Tip: daemon will not survive reboot — run \`heartbeat install --autostart\``
- [x] Ensure `plist_path()` helper is accessible from both files (move to a shared module or re-export from `src/cli/mod.rs` if currently scoped to `install.rs`)
- [x] Add unit test (or integration-style test): mock/stub plist path to a non-existent path, call list output builder, assert hint string appears
- [x] Manual smoke test: run `heartbeat daemon` without having run `install --autostart`; confirm tip line appears in terminal output; run `heartbeat list`; confirm hint line appears below job table
