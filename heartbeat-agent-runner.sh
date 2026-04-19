#!/bin/bash
# Heartbeat agent runner
# Runs an agent in the current user context (Keychain-safe).
#
# Modes:
#   (no flags)        Run once immediately and exit
#   -i/--interval     Run repeatedly, sleeping between runs (daemon)
#   --at HH:MM        Sleep until that time today (or tomorrow if past), run once, exit
#
# Usage: heartbeat-agent-runner.sh [-l logfile] [-i interval | --at HH:MM] <agent> <cwd> <prompt> [params...]
#
# Examples:
#   heartbeat-agent-runner.sh claude ~/myapp "Review changes"
#   heartbeat-agent-runner.sh --at 01:00 -l ~/.heartbeat/logs/nightly.log claude ~/myapp "Nightly review"
#   nohup heartbeat-agent-runner.sh -i 30m -l ~/.heartbeat/logs/myapp.log claude ~/myapp "Review changes" &

parse_interval() {
    local val="$1"
    case "$val" in
        *s) echo "${val%s}" ;;
        *m) echo $(( ${val%m} * 60 )) ;;
        *h) echo $(( ${val%h} * 3600 )) ;;
        *)  echo "$val" ;;
    esac
}

# Seconds until next occurrence of HH:MM (today or tomorrow if already past)
seconds_until() {
    local target="$1"  # HH:MM
    local now target_ts now_ts secs
    now_ts=$(date +%s)
    target_ts=$(date -j -f "%H:%M" "$target" +%s 2>/dev/null) || {
        echo "Invalid time format '$target'. Use HH:MM (e.g. 01:00)" >&2
        exit 1
    }
    secs=$(( target_ts - now_ts ))
    # If target is in the past (even by a second), schedule for tomorrow
    if [[ $secs -le 0 ]]; then
        secs=$(( secs + 86400 ))
    fi
    echo "$secs"
}

log_file=""
interval=""
run_at=""

while [[ "${1:-}" == -* ]]; do
    case "${1:-}" in
        -l)              log_file="$2";                        shift 2 ;;
        -i|--interval)   interval="$(parse_interval "$2")";   shift 2 ;;
        --at)            run_at="$2";                          shift 2 ;;
        *)               break ;;
    esac
done

agent="${1:-}"
requested_cwd="${2:-.}"
prompt="${3:-}"
shift 3 2>/dev/null || true
params=("$@")

if [[ -z "$agent" || -z "$prompt" ]]; then
    echo "Usage: heartbeat-agent-runner.sh [-l logfile] [-i interval | --at HH:MM] <agent> <cwd> <prompt> [params...]" >&2
    exit 1
fi

if [[ -n "$interval" && -n "$run_at" ]]; then
    echo "ERROR: -i/--interval and --at are mutually exclusive" >&2
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

if [[ -n "$run_at" ]]; then
    # One-shot: sleep until HH:MM, run once, exit
    secs="$(seconds_until "$run_at")"
    wake_time="$(date -j -v+${secs}S +"%Y-%m-%d %H:%M:%S" 2>/dev/null || date -d "+${secs} seconds" +"%Y-%m-%d %H:%M:%S")"
    echo "--- $(date) [scheduled: agent=$agent at=$run_at (~${secs}s, wakes $wake_time)] ---"
    sleep "$secs"
    run_once
elif [[ -n "$interval" ]]; then
    # Daemon: run immediately then repeat every interval
    echo "--- $(date) [starting: agent=$agent interval=${interval}s] ---"
    while true; do
        run_once
        echo "--- sleeping ${interval}s ---"
        sleep "$interval"
    done
else
    # Immediate one-shot
    run_once
fi
