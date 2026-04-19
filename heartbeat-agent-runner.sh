#!/bin/bash
# Heartbeat agent runner
# Executes CLI agents with a stable HOME/PATH context (cron-safe).
# Usage: heartbeat-agent-runner.sh [-l logfile] <agent> <cwd> <prompt> [params...]

log_file=""
if [[ "${1:-}" == "-l" ]]; then
    log_file="$2"
    shift 2
fi

agent="${1:-}"
requested_cwd="${2:-.}"
prompt="${3:-}"
shift 3 2>/dev/null || true
params=("$@")

if [[ -z "$agent" || -z "$prompt" ]]; then
    echo "agent and prompt are required" >&2
    exit 1
fi

real_home="$(python3 -c 'import os,pwd; print(pwd.getpwuid(os.getuid()).pw_dir)' 2>/dev/null)"
if [[ -z "$real_home" ]]; then
    real_home="${HOME}"
fi

export HOME="${real_home}"

# Load API keys from ~/.heartbeat/.env (cron has no Keychain access)
# File format: KEY=value, one per line, lines starting with # ignored
env_file="${real_home}/.heartbeat/.env"
if [[ -f "$env_file" ]]; then
    while IFS= read -r line || [[ -n "$line" ]]; do
        [[ "$line" =~ ^#.*$ || -z "$line" ]] && continue
        export "$line"
    done < "$env_file"
fi

extra_paths=(
    "/opt/homebrew/bin"
    "/usr/local/bin"
    "${real_home}/.local/bin"
    "${real_home}/.npm-global/bin"
    "${real_home}/.nvm/versions/node/current/bin"
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
    echo "--- $(date) ---"
fi

run_cwd="${requested_cwd}"
if [[ "$run_cwd" == "~"* ]]; then
    run_cwd="${real_home}${run_cwd:1}"
fi

case "$agent" in
    shell)
        cmd=(bash -c "$prompt")
        ;;
    claude)
        cmd=(claude -p "${params[@]}" "$prompt")
        ;;
    opencode)
        cmd=(opencode run "${params[@]}" "$prompt")
        ;;
    codex)
        cmd=(codex exec "${params[@]}" "$prompt")
        ;;
    *)
        cmd=("$agent" "${params[@]}" "$prompt")
        ;;
esac

if [[ -n "$run_cwd" && "$run_cwd" != "." ]]; then
    if [[ ! -d "$run_cwd" ]]; then
        echo "working directory not found: $run_cwd" >&2
        exit 1
    fi
    (
        cd "$run_cwd" || exit 1
        "${cmd[@]}"
    )
else
    "${cmd[@]}"
fi
