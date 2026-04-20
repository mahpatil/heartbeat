# install-distribution Specification

## Purpose

Allow users to install heartbeat on macOS without having Rust or any build
toolchain installed. Pre-built binaries for `aarch64-apple-darwin` and
`x86_64-apple-darwin` are published to GitHub Releases. An `install.sh`
script handles download, checksum verification, and PATH configuration.

---

## Requirements

### Requirement: Pre-built binary releases

For every git tag matching `v*`, a GitHub Actions workflow SHALL build and
publish two binary assets to the GitHub Release:

- `heartbeat-aarch64-apple-darwin`
- `heartbeat-aarch64-apple-darwin.sha256`
- `heartbeat-x86_64-apple-darwin`
- `heartbeat-x86_64-apple-darwin.sha256`

Binaries SHALL be stripped (`strip`) and compiled with:
```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = "z"
```

Target size: ≤ 3 MB after strip.

#### Scenario: Release workflow triggers on tag
- GIVEN a tag `v0.2.0` is pushed to the repository
- WHEN the GitHub Actions workflow runs
- THEN both binaries and their SHA-256 files are attached to the GitHub Release

---

### Requirement: `install.sh` — platform detection

The install script SHALL detect the host architecture and select the
correct binary asset.

#### Scenario: Apple Silicon Mac
- GIVEN `uname -m` returns `arm64`
- WHEN `install.sh` runs
- THEN it downloads `heartbeat-aarch64-apple-darwin`

#### Scenario: Intel Mac
- GIVEN `uname -m` returns `x86_64`
- WHEN `install.sh` runs
- THEN it downloads `heartbeat-x86_64-apple-darwin`

#### Scenario: Unsupported platform
- GIVEN `uname -m` returns `riscv64`
- WHEN `install.sh` runs
- THEN it prints "Unsupported architecture: riscv64" and exits 1

---

### Requirement: `install.sh` — checksum verification

The install script SHALL download the `.sha256` file alongside the binary
and verify the checksum before installing. If verification fails, the
downloaded file SHALL be deleted and the script exits with a non-zero code.

#### Scenario: Checksum passes
- GIVEN the downloaded binary matches the `.sha256` file
- WHEN `shasum -a 256 -c` is run
- THEN installation proceeds

#### Scenario: Checksum fails
- GIVEN the downloaded binary is corrupted or tampered with
- WHEN `shasum -a 256 -c` is run
- THEN the script prints "Checksum verification failed" and exits 1
- AND the corrupted binary is not installed

---

### Requirement: `install.sh` — directory setup

The script SHALL create `~/.heartbeat/`, `~/.heartbeat/jobs/`, and
`~/.heartbeat/logs/` if they do not exist.

---

### Requirement: `install.sh` — PATH configuration

The script SHALL add `export PATH="$HOME/.heartbeat:$PATH"` to the user's
shell rc file (`~/.zshrc` or `~/.bashrc`) if not already present.

#### Scenario: zshrc updated
- GIVEN macOS with zsh as the default shell and `~/.zshrc` present
- WHEN `install.sh` runs
- THEN `export PATH="$HOME/.heartbeat:$PATH"` is appended to `~/.zshrc`
- AND the script prints "Added ~/.heartbeat to PATH in ~/.zshrc"

#### Scenario: Already in PATH
- GIVEN `~/.zshrc` already contains a heartbeat PATH entry
- WHEN `install.sh` runs
- THEN no duplicate entry is added

---

### Requirement: `install.sh` — agent runner installation

The script SHALL also download `heartbeat-agent-runner.sh` from the
repository's `main` branch and install it to `~/.heartbeat/`, making
it executable.

---

### Requirement: Cargo fallback

If no pre-built binary exists for the detected platform (e.g., Linux),
the script SHALL check for `cargo` in `$PATH` and offer to build from
source with `cargo install --git`.

#### Scenario: No binary, cargo available
- GIVEN the platform has no release binary but `cargo` is in PATH
- WHEN `install.sh` runs
- THEN it prints "No pre-built binary for this platform. Building from source..."
- AND runs `cargo install --git <repo-url>`

#### Scenario: No binary, no cargo
- GIVEN the platform has no release binary and `cargo` is not installed
- WHEN `install.sh` runs
- THEN it prints instructions for installing Rust and exits 1

---

### Requirement: One-liner install

The install script SHALL work via `curl | bash`:
```bash
curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash
```

#### Scenario: One-liner on fresh Mac
- GIVEN macOS with curl available and no prior heartbeat installation
- WHEN the one-liner is executed in a terminal
- THEN heartbeat is installed to `~/.heartbeat/heartbeat`
- AND the terminal prints "Done. Run: heartbeat daemon"
