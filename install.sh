#!/usr/bin/env bash
# Heartbeat installer — no Rust required for end users.
# Usage: curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash
set -euo pipefail

REPO="mahpatil/heartbeat"
INSTALL_DIR="${HEARTBEAT_DIR:-$HOME/.heartbeat}"
BIN="$INSTALL_DIR/heartbeat"
RUNNER="$INSTALL_DIR/heartbeat-agent-runner.sh"
RAW_BASE="https://raw.githubusercontent.com/$REPO/main"

# ── Helpers ───────────────────────────────────────────────────────────────────

info()  { echo "[heartbeat] $*"; }
warn()  { echo "[heartbeat] WARNING: $*" >&2; }
die()   { echo "[heartbeat] ERROR: $*" >&2; exit 1; }

# ── Platform detection ────────────────────────────────────────────────────────

detect_target() {
    local arch os
    arch="$(uname -m)"
    os="$(uname -s)"

    if [[ "$os" == "Darwin" ]]; then
        case "$arch" in
            arm64|aarch64) echo "aarch64-apple-darwin" ;;
            x86_64)        echo "x86_64-apple-darwin"  ;;
            *)             die "Unsupported architecture: $arch" ;;
        esac
    else
        echo "unsupported-$os-$arch"
    fi
}

TARGET="$(detect_target)"

# ── Directory setup ───────────────────────────────────────────────────────────

info "Setting up $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR/jobs" "$INSTALL_DIR/logs"

# ── Fetch latest release tag (best-effort) ────────────────────────────────────

LATEST="$(curl -fsSL --max-time 10 "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null \
    | grep '"tag_name"' | cut -d'"' -f4 || true)"

# ── Binary install ────────────────────────────────────────────────────────────

install_binary() {
    # Try pre-built binary first (macOS only, release must exist)
    if [[ "$TARGET" == *"apple-darwin"* && -n "$LATEST" ]]; then
        local asset_url="https://github.com/$REPO/releases/download/$LATEST/heartbeat-$TARGET"
        local checksum_url="$asset_url.sha256"

        info "Downloading heartbeat $LATEST ($TARGET)..."
        if curl -fsSL --max-time 60 "$asset_url" -o "$BIN.tmp" 2>/dev/null \
        && curl -fsSL --max-time 10 "$checksum_url" -o "$BIN.tmp.sha256" 2>/dev/null; then

            info "Verifying checksum..."
            local expected actual
            expected="$(cat "$BIN.tmp.sha256")"
            actual="$(shasum -a 256 "$BIN.tmp" | awk '{print $1}')"

            if [[ "$expected" != "$actual" ]]; then
                rm -f "$BIN.tmp" "$BIN.tmp.sha256"
                die "Checksum verification failed. Expected: $expected  Got: $actual"
            fi

            mv "$BIN.tmp" "$BIN"
            rm -f "$BIN.tmp.sha256"
            chmod +x "$BIN"
            info "Installed binary: $BIN"
            return 0
        else
            rm -f "$BIN.tmp" "$BIN.tmp.sha256"
            warn "Pre-built binary unavailable — falling back to cargo."
        fi
    elif [[ -z "$LATEST" ]]; then
        warn "No GitHub release found — falling back to cargo."
    else
        warn "No pre-built binary for this platform ($TARGET) — falling back to cargo."
    fi

    # Cargo fallback
    if ! command -v cargo &>/dev/null; then
        echo
        echo "  Rust is not installed. To install it:"
        echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo "  Then re-run this script."
        die "cargo not found."
    fi

    # If we're running from inside the repo, build locally (faster, no network)
    if [[ -f "$(pwd)/Cargo.toml" ]] && grep -q 'name = "heartbeat"' "$(pwd)/Cargo.toml" 2>/dev/null; then
        info "Building from local source..."
        cargo build --release
        cp "$(pwd)/target/release/heartbeat" "$BIN"
    else
        info "Building from source (this may take a few minutes)..."
        cargo install --git "https://github.com/$REPO" --root "$INSTALL_DIR" --bin heartbeat
        # cargo install puts the binary in $INSTALL_DIR/bin/heartbeat
        [[ -f "$INSTALL_DIR/bin/heartbeat" ]] && mv "$INSTALL_DIR/bin/heartbeat" "$BIN" || true
    fi

    chmod +x "$BIN"
    info "Installed binary: $BIN"
}

install_binary

# ── Install agent runner ──────────────────────────────────────────────────────

if [[ -f "$(pwd)/heartbeat-agent-runner.sh" ]]; then
    # Running from repo — copy local version
    cp "$(pwd)/heartbeat-agent-runner.sh" "$RUNNER"
else
    info "Installing heartbeat-agent-runner.sh..."
    curl -fsSL "$RAW_BASE/heartbeat-agent-runner.sh" -o "$RUNNER"
fi
chmod +x "$RUNNER"

# ── PATH configuration ────────────────────────────────────────────────────────

PATH_LINE='export PATH="$HOME/.heartbeat:$PATH"'

for RC in "$HOME/.zshrc" "$HOME/.bashrc" "$HOME/.bash_profile"; do
    if [[ -f "$RC" ]]; then
        if grep -q 'heartbeat' "$RC"; then
            info "PATH already configured in $RC — skipping."
        else
            echo "$PATH_LINE" >> "$RC"
            info "Added ~/.heartbeat to PATH in $RC"
        fi
        break
    fi
done

# ── Done ──────────────────────────────────────────────────────────────────────

VERSION="$("$BIN" --version 2>/dev/null || echo "${LATEST:-dev}")"
echo
echo "  $VERSION installed to $INSTALL_DIR"
echo
echo "  Next steps:"
echo "    1. Reload your shell:  source ~/.zshrc   (or open a new terminal)"
echo "    2. Start the daemon:   heartbeat daemon"
echo "    3. Apply a job:        heartbeat apply my-job.htb"
echo "    4. Optional autostart: heartbeat install --autostart"
echo
echo "Done. Run: heartbeat daemon"
