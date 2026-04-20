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
    local arch
    arch="$(uname -m)"
    local os
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

info "Setting up ~/.heartbeat..."
mkdir -p "$INSTALL_DIR/jobs" "$INSTALL_DIR/logs"

# ── Fetch latest release tag ──────────────────────────────────────────────────

LATEST="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
    | grep '"tag_name"' | cut -d'"' -f4)"

if [[ -z "$LATEST" ]]; then
    die "Could not determine latest release from GitHub API."
fi

# ── macOS: download pre-built binary ─────────────────────────────────────────

if [[ "$TARGET" == *"apple-darwin"* ]]; then
    ASSET_URL="https://github.com/$REPO/releases/download/$LATEST/heartbeat-$TARGET"
    CHECKSUM_URL="$ASSET_URL.sha256"

    info "Downloading heartbeat $LATEST ($TARGET)..."
    curl -fsSL "$ASSET_URL"     -o "$BIN.tmp"
    curl -fsSL "$CHECKSUM_URL"  -o "$BIN.tmp.sha256"

    # Verify checksum
    info "Verifying checksum..."
    EXPECTED="$(cat "$BIN.tmp.sha256")"
    ACTUAL="$(shasum -a 256 "$BIN.tmp" | awk '{print $1}')"

    if [[ "$EXPECTED" != "$ACTUAL" ]]; then
        rm -f "$BIN.tmp" "$BIN.tmp.sha256"
        die "Checksum verification failed. Expected: $EXPECTED  Got: $ACTUAL"
    fi

    mv "$BIN.tmp" "$BIN"
    rm -f "$BIN.tmp.sha256"
    chmod +x "$BIN"
    info "Installed binary: $BIN"

# ── Non-macOS: cargo fallback ─────────────────────────────────────────────────

else
    warn "No pre-built binary for this platform ($TARGET)."

    if command -v cargo &>/dev/null; then
        info "Building from source (this may take a few minutes)..."
        cargo install --git "https://github.com/$REPO" --root "$INSTALL_DIR" heartbeat
    else
        echo
        echo "  To install Rust: https://rustup.rs"
        echo "  Then re-run this script."
        die "cargo not found. Please install Rust first."
    fi
fi

# ── Install agent runner ──────────────────────────────────────────────────────

info "Installing heartbeat-agent-runner.sh..."
curl -fsSL "$RAW_BASE/heartbeat-agent-runner.sh" -o "$RUNNER"
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

echo
echo "  Heartbeat $LATEST installed to $INSTALL_DIR"
echo
echo "  Next steps:"
echo "    1. Reload your shell:  source ~/.zshrc   (or open a new terminal)"
echo "    2. Start the daemon:   heartbeat daemon"
echo "    3. Apply a job:        heartbeat apply my-job.htb"
echo "    4. Optional autostart: heartbeat install --autostart"
echo
echo "Done. Run: heartbeat daemon"
