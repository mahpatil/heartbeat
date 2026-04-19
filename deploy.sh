#!/bin/bash
#
# Deploy heartbeat to ~/.heartbeat
# Usage: ./deploy.sh
#

set -e

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HEARTBEAT_DIR="${HOME}/.heartbeat"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[+]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[!]${NC} $1"; }

log_info "Deploying from $REPO_DIR to $HEARTBEAT_DIR"

# Ensure directories exist
mkdir -p "$HEARTBEAT_DIR/plugins" "$HEARTBEAT_DIR/jobs" "$HEARTBEAT_DIR/logs"

# Core Python script
cp "$REPO_DIR/heartbeat.py" "$HEARTBEAT_DIR/heartbeat.py"
chmod +x "$HEARTBEAT_DIR/heartbeat.py"
log_info "Copied heartbeat.py"

# Bash CLI and any other shell scripts in repo root
for script in "$REPO_DIR"/heartbeat "$REPO_DIR"/*.sh; do
    [[ -f "$script" ]] || continue
    fname="$(basename "$script")"
    cp "$script" "$HEARTBEAT_DIR/$fname"
    chmod +x "$HEARTBEAT_DIR/$fname"
    log_info "Copied $fname"
done

# Plugins
if compgen -G "$REPO_DIR/plugins/*.py" > /dev/null 2>&1; then
    cp "$REPO_DIR/plugins/"*.py "$HEARTBEAT_DIR/plugins/"
    log_info "Copied plugins"
fi

log_info "Deploy complete"
