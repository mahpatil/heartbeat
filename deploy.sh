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

# Core files
cp "$REPO_DIR/heartbeat.py" "$HEARTBEAT_DIR/heartbeat.py"
cp "$REPO_DIR/heartbeat" "$HEARTBEAT_DIR/heartbeat"
chmod +x "$HEARTBEAT_DIR/heartbeat.py" "$HEARTBEAT_DIR/heartbeat"
log_info "Copied heartbeat.py and heartbeat CLI"

# Plugins
cp "$REPO_DIR/plugins/"*.py "$HEARTBEAT_DIR/plugins/"
log_info "Copied plugins"

log_info "Deploy complete"
