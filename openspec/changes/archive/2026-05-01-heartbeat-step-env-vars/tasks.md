## Tasks

- [x] Add `env: HashMap<String, String>` field to `StepDef::Shell` and `StepDef::Agent` variants in `src/task/types.rs`; add `use std::collections::HashMap`; default to empty map (not `Option`)
- [x] Add `env: Option<HashMap<String, String>>` to `RawStep` in `src/job/config.rs`; in `raw_step_to_def` map it to the step's `env` field using `.unwrap_or_default()`
- [x] In `src/task/executor.rs`: for both `execute_shell` and `execute_agent`, iterate over the step's `env` map and call `.env(k, v)` on the `tokio::process::Command` before `.spawn()`
- [x] Update all `StepDef::Shell { ..., timeout: None }` and `StepDef::Agent { ..., timeout: None }` literals in tests across `src/task/executor.rs`, `src/job/config.rs`, and `src/job/runner.rs` to add `env: Default::default()`
- [x] Add unit test in `src/task/executor.rs`: shell step with `env: [("MY_VAR", "hello")]` running `echo $MY_VAR` produces output `hello`
- [x] Add unit test in `src/job/config.rs`: YAML with `env: { FOO: bar, NUM: "42" }` parses to step with `env = {"FOO": "bar", "NUM": "42"}`
- [x] Update `~/.heartbeat/jobs/algo-scout.htb` (or `.yaml`) to add `env: { OPENCODE_BASH_TIMEOUT_MS: "120000" }` to the agent step that runs opencode
- [x] Run `cargo test` — all tests pass; run `cargo clippy` — no warnings
