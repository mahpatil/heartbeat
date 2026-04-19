#!/bin/bash
# Heartbeat agent runner
# Runs an agent on a repeating interval in the current user context (Keychain-safe).
# Usage: heartbeat-agent-runner.sh [-l logfile] [-i interval] <agent> <cwd> <prompt> [params...]
#
# Interval examples: 30s, 15m, 2h, 3600 (bare number = seconds). Default: 15m
# Agent values:      claude, opencode, codex, shell
#
# Example (foreground):
#   heartbeat-agent-runner.sh -i 30m -l ~/.heartbeat/logs/myapp.log claude ~/myapp "Review changes"
#
# Example (background, survives terminal close):
#   nohup heartbeat-agent-runner.sh -i 1h claude ~ "Daily summary" >> ~/.heartbeat/logs/summary.log 2>&1 &

parse_interval() {
    local val="$1"
    case "$val" in
        *s) echo "${val%s}" ;;
        *m) echo $(( ${val%m} * 60 )) ;;
        *h) echo $(( ${val%h} * 3600 )) ;;
        *)  echo "$val" ;;
    esac
}

log_file=""
interval=900  # default 15 minutes

while [[ "${1:-}" == -* ]]; do
    case "${1:-}" in
        -l)          log_file="$2";                          shift 2 ;;
        -i|--interval) interval="$(parse_interval "$2")";   shift 2 ;;
        *)           break ;;
    esac
done

agent="${1:-}"
requested_cwd="${2:-.}"
prompt="${3:-}"
shift 3 2>/dev/null || true
params=("$@")

if [[ -z "$agent" || -z "$prompt" ]]; then
    echo "Usage: heartbeat-agent-runner.sh [-l logfile] [-i interval] <agent> <cwd> <prompt> [params...]" >&2
    exit 1
fi

# Enrich PATH for common tool locations (useful when launched via launchd)
extra_paths=(
    "/opt/homebrew/bin"
    "/usr/local/bin"
    "${HOME}/.local/bin"
    "${HOME}/.npm-global/bin"
    "${HOME}/.nvm/versions/node/current/bin"
)
for p in "${extra_paths[@]}"; do
    case ":${PATH:-}:" in
        *":${p}:"*) ;;
        *) PATH="${p}${PATH:+:${PATH}}" ;;
    esac
done
export PATH

if [[ -n "$log_file" ]]; then
    mkdir -p "$(dirname "$log_file")"
    exec >> "$log_file" 2>&1
fi

run_cwd="${requested_cwd}"
if [[ "$run_cwd" == "~"* ]]; then
    run_cwd="${HOME}${run_cwd:1}"
fi

case "$agent" in
    shell)    cmd=(bash -c "$prompt") ;;
    claude)   cmd=(claude -p "${params[@]}" "$prompt") ;;
    opencode) cmd=(opencode run "${params[@]}" "$prompt") ;;
    codex)    cmd=(codex exec "${params[@]}" "$prompt") ;;
    *)        cmd=("$agent" "${params[@]}" "$prompt") ;;
esac

run_once() {
    echo "--- $(date) [agent=$agent cwd=${run_cwd:-.}] ---"
    if [[ -n "$run_cwd" && "$run_cwd" != "." ]]; then
        if [[ ! -d "$run_cwd" ]]; then
            echo "ERROR: working directory not found: $run_cwd" >&2
            return 1
        fi
        (cd "$run_cwd" && "${cmd[@]}")
    else
        "${cmd[@]}"
    fi
}

trap 'echo "--- $(date) [stopped] ---"; exit 0' SIGTERM SIGINT

echo "--- $(date) [starting: agent=$agent interval=${interval}s] ---"
while true; do
    run_once
    echo "--- sleeping ${interval}s ---"
    sleep "$interval"
done
