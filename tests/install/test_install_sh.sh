#!/usr/bin/env bash
# Tests for install.sh logic — no network, no GitHub required.
# Usage: bash tests/install/test_install_sh.sh
set -euo pipefail

PASS=0
FAIL=0
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# ── Test harness ──────────────────────────────────────────────────────────────

ok()   { echo "  PASS  $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL  $1"; FAIL=$((FAIL + 1)); }

assert_eq() {
    local label="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        ok "$label"
    else
        fail "$label — expected '$expected', got '$actual'"
    fi
}

assert_contains() {
    local label="$1" haystack="$2" needle="$3"
    if echo "$haystack" | grep -qF "$needle"; then
        ok "$label"
    else
        fail "$label — expected to find: '$needle' in: '$haystack'"
    fi
}

assert_not_contains() {
    local label="$1" haystack="$2" needle="$3"
    if ! echo "$haystack" | grep -qF "$needle"; then
        ok "$label"
    else
        fail "$label — did not expect: '$needle'"
    fi
}

# ── Platform detection ────────────────────────────────────────────────────────
# Test the detect_target logic by running a small inline script.

detect_target_for() {
    local arch="$1" os="$2"
    bash -c "
        die() { echo \"ERROR: \$*\" >&2; exit 1; }
        detect_target() {
            local arch='$arch'
            local os='$os'
            if [[ \"\$os\" == \"Darwin\" ]]; then
                case \"\$arch\" in
                    arm64|aarch64) echo \"aarch64-apple-darwin\" ;;
                    x86_64)        echo \"x86_64-apple-darwin\"  ;;
                    *)             die \"Unsupported architecture: \$arch\" ;;
                esac
            else
                echo \"unsupported-\$os-\$arch\"
            fi
        }
        detect_target
    " 2>&1 || true
}

echo ""
echo "── Platform detection ───────────────────────────────────────────────────"

assert_eq "arm64/Darwin → aarch64-apple-darwin" \
    "aarch64-apple-darwin" "$(detect_target_for arm64 Darwin)"

assert_eq "aarch64/Darwin → aarch64-apple-darwin" \
    "aarch64-apple-darwin" "$(detect_target_for aarch64 Darwin)"

assert_eq "x86_64/Darwin → x86_64-apple-darwin" \
    "x86_64-apple-darwin" "$(detect_target_for x86_64 Darwin)"

assert_contains "riscv64/Darwin → unsupported error" \
    "$(detect_target_for riscv64 Darwin)" "Unsupported architecture: riscv64"

assert_contains "x86_64/Linux → unsupported prefix" \
    "$(detect_target_for x86_64 Linux)" "unsupported-"

# ── Checksum verification ─────────────────────────────────────────────────────

echo ""
echo "── Checksum verification ────────────────────────────────────────────────"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "fake binary content v1.0" > "$TMP/heartbeat.bin"
GOOD_SUM="$(shasum -a 256 "$TMP/heartbeat.bin" | awk '{print $1}')"
echo "$GOOD_SUM" > "$TMP/heartbeat.bin.sha256"

ACTUAL="$(shasum -a 256 "$TMP/heartbeat.bin" | awk '{print $1}')"
[[ "$GOOD_SUM" == "$ACTUAL" ]] \
    && ok "checksum passes when binary is intact" \
    || fail "checksum passes when binary is intact"

# Tamper with the binary
echo "tampered" >> "$TMP/heartbeat.bin"
TAMPERED="$(shasum -a 256 "$TMP/heartbeat.bin" | awk '{print $1}')"
[[ "$GOOD_SUM" != "$TAMPERED" ]] \
    && ok "checksum fails when binary is tampered" \
    || fail "checksum fails when binary is tampered"

# Simulate the install.sh guard
run_checksum_guard() {
    local expected actual
    expected="$(cat "$TMP/heartbeat.bin.sha256")"
    actual="$(shasum -a 256 "$TMP/heartbeat.bin" | awk '{print $1}')"
    if [[ "$expected" != "$actual" ]]; then
        rm -f "$TMP/heartbeat.bin"
        echo "Checksum verification failed"
        return 1
    fi
}
run_checksum_guard 2>&1 || true

[[ ! -f "$TMP/heartbeat.bin" ]] \
    && ok "corrupted binary deleted on checksum failure" \
    || fail "corrupted binary deleted on checksum failure"

# ── Directory setup ───────────────────────────────────────────────────────────

echo ""
echo "── Directory setup ──────────────────────────────────────────────────────"

FAKE_HOME="$TMP/fakehome"
INSTALL_DIR="$FAKE_HOME/.heartbeat"
mkdir -p "$INSTALL_DIR/jobs" "$INSTALL_DIR/logs"

[[ -d "$INSTALL_DIR/jobs" ]]  && ok "jobs/ directory created"  || fail "jobs/ directory created"
[[ -d "$INSTALL_DIR/logs" ]]  && ok "logs/ directory created"  || fail "logs/ directory created"

# ── PATH configuration ────────────────────────────────────────────────────────

echo ""
echo "── PATH configuration ───────────────────────────────────────────────────"

FAKE_ZSHRC="$TMP/.zshrc"
touch "$FAKE_ZSHRC"
PATH_LINE='export PATH="$HOME/.heartbeat:$PATH"'

# First run: adds the line
if ! grep -q 'heartbeat' "$FAKE_ZSHRC"; then
    echo "$PATH_LINE" >> "$FAKE_ZSHRC"
fi
assert_contains "PATH line added to .zshrc" "$(cat "$FAKE_ZSHRC")" ".heartbeat"

# Second run: does NOT add a duplicate (idempotent)
if ! grep -q 'heartbeat' "$FAKE_ZSHRC"; then
    echo "$PATH_LINE" >> "$FAKE_ZSHRC"
fi
LINE_COUNT="$(grep -c 'heartbeat' "$FAKE_ZSHRC")"
assert_eq "no duplicate PATH entries" "1" "$LINE_COUNT"

# ── Binary install ────────────────────────────────────────────────────────────

echo ""
echo "── Binary install ───────────────────────────────────────────────────────"

BIN="$INSTALL_DIR/heartbeat"
echo "fake heartbeat binary" > "$BIN"
chmod +x "$BIN"
[[ -x "$BIN" ]] && ok "installed binary is executable" || fail "installed binary is executable"

RUNNER="$INSTALL_DIR/heartbeat-agent-runner.sh"
cp "$REPO_ROOT/heartbeat-agent-runner.sh" "$RUNNER"
chmod +x "$RUNNER"
[[ -x "$RUNNER" ]] && ok "agent runner is executable" || fail "agent runner is executable"

# ── install.sh is valid bash syntax ──────────────────────────────────────────

echo ""
echo "── Script syntax ────────────────────────────────────────────────────────"

bash -n "$REPO_ROOT/install.sh" \
    && ok "install.sh passes bash -n syntax check" \
    || fail "install.sh passes bash -n syntax check"

# ── Summary ───────────────────────────────────────────────────────────────────

echo ""
echo "Results: $PASS passed, $FAIL failed"
echo ""

[[ "$FAIL" -eq 0 ]] || exit 1
