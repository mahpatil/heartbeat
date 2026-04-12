#!/bin/bash
#
# Heartbeat Install Script
# Usage: curl -fsSL https://raw.githubusercontent.com/mahpatil/heartbeat/main/install.sh | bash
#

set -e

HEARTBEAT_DIR="${HOME}/.heartbeat"
REPO_URL="https://github.com/mahpatil/heartbeat"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[+]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[!]${NC} $1"; }
log_error() { echo -e "${RED}[-]${NC} $1"; }

echo "Heartbeat Installer"
echo "==============="

# Check Python
if ! command -v python3 &> /dev/null; then
    log_error "Python 3 not found. Please install Python 3 first."
    exit 1
fi
log_info "Python 3 found: $(python3 --version)"

# Create directory
mkdir -p "${HEARTBEAT_DIR}/jobs" "${HEARTBEAT_DIR}/logs"

# Download files
log_info "Downloading heartbeat..."

if command -v curl &> /dev/null; then
    DOWNLOAD="curl -fsSL"
elif command -v wget &> /dev/null; then
    DOWNLOAD="wget -qO -"
else
    log_error "curl or wget required"
    exit 1
fi

# Get raw URL
RAW_URL="${REPO_URL}/raw/main/heartbeat"

# Download heartbeat.py
${DOWNLOAD} "${RAW_URL}/heartbeat.py" > "${HEARTBEAT_DIR}/heartbeat.py" || {
    log_error "Failed to download heartbeat.py"
    exit 1
}

# Download CLI wrapper
${DOWNLOAD} "${RAW_URL}/heartbeat" > "${HEARTBEAT_DIR}/heartbeat" || {
    log_error "Failed to download heartbeat"
    exit 1
}

chmod +x "${HEARTBEAT_DIR}/heartbeat.py" "${HEARTBEAT_DIR}/heartbeat"

# Add to PATH
SHELL_RC="${HOME}/.zshrc"
if [[ "$SHELL" == *bash* ]]; then
    SHELL_RC="${HOME}/.bashrc"
fi

PATH_LINE='export PATH="$HOME/.heartbeat:$PATH"'

if [[ -f "$SHELL_RC" ]]; then
    if ! grep -q "heartbeat" "$SHELL_RC"; then
        echo "" >> "$SHELL_RC"
        echo "# Heartbeat" >> "$SHELL_RC"
        echo "$PATH_LINE" >> "$SHELL_RC"
        log_info "Added to $SHELL_RC"
    fi
else
    echo "$PATH_LINE" >> "$SHELL_RC"
    log_info "Created $SHELL_RC"
fi

# Create sample job
if [[ ! -f "${HEARTBEAT_DIR}/jobs/sample.yaml" ]]; then
    cat > "${HEARTBEAT_DIR}/jobs/sample.yaml" << 'EOF'
name: "sample"
folder: "~"
frequency: "*/15 * * * *"

tasks:
- type: run
  command: "echo 'Heartbeat running at $(date)'"
EOF
    log_info "Created sample job"
fi

log_info "Installation complete!"
echo ""
echo "Run these commands to get started:"
echo ""
echo "  source $SHELL_RC"
echo "  heartbeat list"
echo "  heartbeat run sample"
echo "  heartbeat add-cron sample"
echo ""