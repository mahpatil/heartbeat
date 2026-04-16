#!/usr/bin/env python3
"""Heartbeat - scheduled task runner with config as code"""

import argparse
import sys
import subprocess
import time
import logging
import pathlib
import re
import json
import urllib.request
import urllib.error
import os
from datetime import datetime

try:
    import yaml

    YAML_AVAILABLE = True
except ImportError:
    YAML_AVAILABLE = False

try:
    from .plugins import get_plugin

    PLUGINS_AVAILABLE = True
except ImportError:
    PLUGINS_AVAILABLE = False

    def get_plugin(name):
        return None


def parse_simple_yaml(content: str) -> dict:
    """Simple YAML parser for basic configs"""
    result = {"tasks": [], "notifications": []}
    tasks = result["tasks"]
    notifications = result["notifications"]
    current_task = None
    current_notification = None
    indent_level = 0

    for line in content.split("\n"):
        original = line
        stripped = line.strip()

        if not stripped or stripped.startswith("#"):
            continue

        indent = len(original) - len(original.lstrip())

        if stripped.startswith("-"):
            stripped = stripped.lstrip("- ").strip()

            if ":" not in stripped:
                continue

            key, _, value = stripped.partition(":")
            key = key.strip()
            value = value.strip().strip('"')

            if key in ("type", "command", "url", "path", "on_fail", "agent", "prompt"):
                if "type" in (key, value) and value in (
                    "run",
                    "url",
                    "file_exists",
                    "agent",
                    "claude",
                    "opencode",
                    "codex",
                    "agent_api",
                ):
                    if current_task:
                        tasks.append(current_task)
                    if current_notification:
                        notifications.append(current_notification)
                        current_notification = None
                    current_task = {"type": value}
                elif key == "type" and value == "telegram":
                    if current_task:
                        tasks.append(current_task)
                        current_task = None
                    if current_notification:
                        notifications.append(current_notification)
                    current_notification = {"type": value}
                elif (
                    key in ("api_token", "chat_id", "on_failure", "on_success")
                    and current_notification
                ):
                    current_notification[key] = value
                elif current_task is None and key in (
                    "name",
                    "folder",
                    "frequency",
                    "run_once_at",
                ):
                    result[key] = value
                elif current_task:
                    current_task[key] = value
            elif current_notification:
                current_notification[key] = value
            continue

        if ":" not in stripped:
            continue

        key, _, value = stripped.partition(":")
        key = key.strip()
        value = value.strip().strip('"')

        if key in ("name", "folder", "frequency", "run_once_at"):
            result[key] = value
        elif key == "type":
            if value == "telegram":
                if current_task:
                    tasks.append(current_task)
                    current_task = None
                if current_notification:
                    notifications.append(current_notification)
                current_notification = {"type": value}
            else:
                if current_notification:
                    notifications.append(current_notification)
                    current_notification = None
                if current_task:
                    tasks.append(current_task)
                current_task = {"type": value}
                tasks.append(current_task)
        elif key in (
            "command",
            "url",
            "path",
            "on_fail",
            "agent",
            "prompt",
            "provider",
            "model",
            "api_key_env",
        ):
            if current_task:
                current_task[key] = value
        elif key in ("api_token", "chat_id", "on_failure", "on_success"):
            if current_notification:
                current_notification[key] = value

    if current_task:
        tasks.append(current_task)
    if current_notification:
        notifications.append(current_notification)

    return result


logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
)
logger = logging.getLogger("heartbeat")


class TaskRunner:
    def run_task(self, task: dict, context: dict) -> bool:
        task_type = task.get("type") or task.get("agent")

        if task_type == "run":
            return self._run_shell(task.get("command", ""), context)
        elif task_type == "url":
            return self._check_url(task.get("url", ""), context)
        elif task_type == "file_exists":
            return self._check_file(task.get("path", ""), context)
        elif task_type in ("agent", "claude", "opencode", "codex"):
            return self._run_agent(task, context)
        elif task_type == "agent_api":
            return self._run_agent_api(task, context)
        else:
            logger.warning(f"Unknown task type: {task_type}")
            return False

    def _run_shell(self, cmd: str, ctx: dict) -> bool:
        if not cmd:
            return False
        try:
            result = subprocess.run(cmd, shell=True, capture_output=True, timeout=300)
            if result.returncode == 0:
                logger.info(f"Command succeeded: {cmd[:50]}...")
            else:
                logger.error(
                    f"Command failed: {cmd[:50]}... -> {result.stderr.decode()[:100]}"
                )
            return result.returncode == 0
        except subprocess.TimeoutExpired:
            logger.error(f"Command timeout: {cmd[:50]}...")
            return False
        except Exception as e:
            logger.error(f"Shell error: {e}")
            return False

    def _check_url(self, url: str, ctx: dict) -> bool:
        if not url:
            return False
        try:
            req = urllib.request.Request(url)
            with urllib.request.urlopen(req, timeout=30) as resp:
                logger.info(f"URL OK: {url}")
                return True
        except urllib.error.HTTPError as e:
            logger.error(f"URL error {url}: {e.code}")
            return False
        except Exception as e:
            logger.error(f"URL failed {url}: {e}")
            return False

    def _check_file(self, path: str, ctx: dict) -> bool:
        if not path:
            return False
        folder = ctx.get("folder", "")
        full_path = pathlib.Path(folder) / path
        exists = full_path.exists()
        if exists:
            logger.info(f"File exists: {full_path}")
        else:
            logger.warning(f"File missing: {full_path}")
        return exists

    _UUID_RE = re.compile(
        r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", re.I
    )
    _SESSIONS_FILE = pathlib.Path.home() / ".heartbeat" / "sessions.json"

    def _load_sessions(self) -> dict:
        if self._SESSIONS_FILE.exists():
            try:
                return json.loads(self._SESSIONS_FILE.read_text())
            except Exception:
                pass
        return {}

    def _save_session(self, name: str, uuid: str) -> None:
        sessions = self._load_sessions()
        sessions[name] = uuid
        try:
            self._SESSIONS_FILE.parent.mkdir(parents=True, exist_ok=True)
            self._SESSIONS_FILE.write_text(json.dumps(sessions, indent=2))
            logger.info(f"Saved session '{name}' -> {uuid}")
        except Exception as e:
            logger.warning(f"Could not save session mapping: {e}")

    def _resolve_claude_params(self, params: list, cwd: str) -> tuple:
        """Resolve --resume <name> to --resume <uuid> or --name <name>.

        Returns (resolved_params, session_name_to_capture | None).
        session_name_to_capture is set when a new named session is being started
        so the caller can save the UUID after a successful run.
        """
        for i, p in enumerate(params):
            if p in ("--resume", "-r") and i + 1 < len(params):
                name = params[i + 1]
                if self._UUID_RE.match(name):
                    return params, None  # already a UUID, nothing to do
                sessions = self._load_sessions()
                if name in sessions:
                    uuid = sessions[name]
                    logger.info(f"Resolved session name '{name}' -> {uuid}")
                    return params[:i] + ["--resume", uuid] + params[i + 2 :], None
                else:
                    # First run: start a new session with this display name
                    logger.info(f"No saved session for '{name}', starting new named session")
                    return params[:i] + ["--name", name] + params[i + 2 :], name
        return params, None

    def _capture_session_uuid(self, name: str, cwd: str) -> None:
        """Find the most recently modified session JSONL for cwd and save name->uuid."""
        encoded = cwd.replace("/", "-") if cwd else None
        if not encoded:
            return
        projects_dir = pathlib.Path.home() / ".claude" / "projects"
        proj_dir = projects_dir / encoded
        if not proj_dir.exists():
            return
        jsonl_files = sorted(proj_dir.glob("*.jsonl"), key=lambda f: f.stat().st_mtime, reverse=True)
        if jsonl_files:
            uuid = jsonl_files[0].stem
            if self._UUID_RE.match(uuid):
                self._save_session(name, uuid)

    def _run_agent(self, task: dict, ctx: dict) -> bool:
        import shlex
        import shutil

        agent = task.get("agent") or task.get("name") or task.get("type")
        prompt = task.get("prompt", "")
        folder = ctx.get("folder", ".")
        params_str = task.get("params") or task.get("args", "")

        if not agent or not prompt:
            logger.error("Agent task missing agent name or prompt")
            return False

        params = shlex.split(params_str) if params_str else []

        cwd = str(pathlib.Path(folder).expanduser()) if folder and folder != "." else None

        capture_session_name = None
        if agent == "claude":
            params, capture_session_name = self._resolve_claude_params(params, cwd or "")
            cmd = ["claude", "-p"] + params + [prompt]
        elif agent == "opencode":
            cmd = ["opencode", "run"] + params + [prompt]
        elif agent == "codex":
            cmd = ["codex", "exec"] + params + [prompt]
        else:
            cmd = [agent] + params + [prompt]

        # Augment PATH so cron environments can find binaries installed via
        # Homebrew, nvm, npm global, etc.
        extra_paths = [
            "/opt/homebrew/bin",
            "/usr/local/bin",
            str(pathlib.Path.home() / ".local" / "bin"),
            str(pathlib.Path.home() / ".npm-global" / "bin"),
            str(pathlib.Path.home() / ".nvm" / "versions" / "node" / "current" / "bin"),
        ]
        env = os.environ.copy()
        current_path = env.get("PATH", "")
        env["PATH"] = ":".join(p for p in extra_paths if p not in current_path) + (
            ":" + current_path if current_path else ""
        )

        binary = cmd[0]
        resolved = shutil.which(binary, path=env["PATH"])
        if resolved:
            cmd[0] = resolved

        logger.info(f"Running agent: {agent} in {cwd or '.'}: {cmd}")

        try:
            result = subprocess.run(cmd, capture_output=True, timeout=600, cwd=cwd, env=env)
            stdout = result.stdout.decode(errors="replace").strip()
            stderr = result.stderr.decode(errors="replace").strip()
            if stdout:
                for line in stdout.splitlines():
                    logger.info(f"[{agent}] {line}")
            if result.returncode == 0:
                logger.info(f"Agent {agent} succeeded")
                if capture_session_name and cwd:
                    self._capture_session_uuid(capture_session_name, cwd)
                return True
            else:
                if stderr:
                    for line in stderr.splitlines():
                        logger.error(f"[{agent}] {line}")
                logger.error(f"Agent {agent} failed (exit {result.returncode})")
                return False
        except FileNotFoundError:
            logger.error(f"Agent not found: {agent}")
            return False
        except subprocess.TimeoutExpired:
            logger.error(f"Agent timeout: {agent}")
            return False
        except Exception as e:
            logger.error(f"Agent error: {e}")
            return False

    def _run_agent_api(self, task: dict, ctx: dict) -> bool:
        provider = task.get("provider", "anthropic")
        model = task.get("model", "claude-sonnet-4-20250514")
        prompt = task.get("prompt", "")

        if not prompt:
            logger.error("API task missing prompt")
            return False

        if provider == "anthropic":
            key_env = task.get("api_key_env", "ANTHROPIC_API_KEY")
            api_key = os.environ.get(key_env)
            if not api_key:
                logger.error(f"Missing API key: {key_env}")
                return False

            try:
                import anthropic

                client = anthropic.Anthropic(api_key=api_key)
                client.messages.create(
                    model=model,
                    max_tokens=1024,
                    messages=[{"role": "user", "content": prompt}],
                )
                logger.info(f"API request succeeded: {provider}/{model}")
                return True
            except ImportError:
                logger.error("anthropic package not installed")
                return False
            except Exception as e:
                logger.error(f"API error: {e}")
                return False

        elif provider == "openai":
            key_env = task.get("api_key_env", "OPENAI_API_KEY")
            api_key = os.environ.get(key_env)
            if not api_key:
                logger.error(f"Missing API key: {key_env}")
                return False

            try:
                import openai

                client = openai.OpenAI(api_key=api_key)
                client.chat.completions.create(
                    model=model, messages=[{"role": "user", "content": prompt}]
                )
                logger.info(f"API request succeeded: {provider}/{model}")
                return True
            except ImportError:
                logger.error("openai package not installed")
                return False
            except Exception as e:
                logger.error(f"API error: {e}")
                return False

        logger.error(f"Unknown provider: {provider}")
        return False


class ConfigParser:
    NL_FREQ_PATTERNS = {
        r"every\s+(\d+)\s*min": "*/{0} * * * *",
        r"every\s+(\d+)\s*minutes": "*/{0} * * * *",
        r"every\s+hour": "0 * * * *",
        r"hourly": "0 * * * *",
        r"daily\s+at\s+(\d{1,2}):(\d{2})": "{1} {0} * * *",
        r"daily\s+at\s+(\d{1,2})\s*am": "{0} * * * *",
        r"daily\s+at\s+(\d{1,2})\s*pm": "{0} * * * *" if False else None,
        r"daily": "0 * * * *",
        r"weekly": "0 0 * * 0",
        r"weekly\s+on\s+monday": "0 0 * * 1",
    }

    def parse_file(self, path: str) -> dict:
        p = pathlib.Path(path)

        if p.suffix in (".yaml", ".yml"):
            return self._parse_yaml(path)
        elif p.suffix == ".htb":
            return self._parse_nl(path)
        else:
            try:
                return self._parse_yaml(path)
            except:
                return self._parse_nl(path)

    def _parse_yaml(self, path: str) -> dict:
        with open(path) as f:
            content = f.read()

        if YAML_AVAILABLE:
            try:
                result = yaml.safe_load(content)
                return result if result else {}
            except:
                pass

        return parse_simple_yaml(content)

    def _parse_nl(self, path: str) -> dict:
        with open(path) as f:
            content = f.read()

        name_match = re.search(r"# Heartbeat:\s*(.+)", content, re.IGNORECASE)
        folder_match = re.search(r"# Folder:\s*(.+)", content, re.IGNORECASE)
        freq_match = re.search(r"Every\s+(.+?):", content, re.IGNORECASE)
        run_once_match = re.search(r"# Run once at:\s*(.+)", content, re.IGNORECASE)

        config = {
            "name": name_match.group(1).strip() if name_match else "Unnamed",
            "folder": folder_match.group(1).strip() if folder_match else ".",
            "frequency": self._parse_frequency(freq_match.group(1))
            if freq_match
            else "*/15 * * * *",
            "run_once_at": run_once_match.group(1).strip() if run_once_match else None,
            "tasks": [],
        }

        in_task_block = False
        current_task = None

        for line in content.split("\n"):
            stripped = line.strip()

            if stripped.startswith("Every ") and ":" in stripped:
                in_task_block = True
                continue

            if not in_task_block:
                continue

            stripped = stripped.lstrip("- ").strip()

            if not stripped:
                continue

            if ":" not in stripped:
                continue

            key, _, value = stripped.partition(":")
            key = key.strip().lower()
            value = value.strip()

            if key in ("url reachable", "url", "check url"):
                if current_task:
                    config["tasks"].append(current_task)
                current_task = {"type": "url", "url": value}
            elif key in ("file exists", "check file", "file"):
                if current_task:
                    config["tasks"].append(current_task)
                current_task = {"type": "file_exists", "path": value}
            elif key == "run":
                if current_task:
                    config["tasks"].append(current_task)
                current_task = {"type": "run", "command": value}
            elif key in ("on missing", "on fail", "on_error"):
                if current_task:
                    current_task["on_fail"] = value
            elif "ask claude" in key or key == "claude":
                if current_task:
                    config["tasks"].append(current_task)
                current_task = {"agent": "claude", "prompt": value}
            elif "ask opencode" in key or key == "opencode":
                if current_task:
                    config["tasks"].append(current_task)
                current_task = {"agent": "opencode", "prompt": value}
            elif "ask codex" in key or key == "codex":
                if current_task:
                    config["tasks"].append(current_task)
                current_task = {"agent": "codex", "prompt": value}
            else:
                continue

        if current_task:
            config["tasks"].append(current_task)

        return config

    def _parse_frequency(self, text: str) -> str:
        text = text.strip().lower()

        patterns = [
            (r"every\s+(\d+)\s*min", "*/{0} * * * *"),
            (r"every\s+(\d+)\s*minutes", "*/{0} * * * *"),
            (r"every\s+hour", "0 * * * *"),
            (r"hourly", "0 * * * *"),
            (r"daily\s+at\s+(\d{1,2}):(\d{2})", "{1} {0} * * *"),
            (r"daily", "0 * * * *"),
            (r"weekly\s+on\s+monday", "0 0 * * 1"),
            (r"weekly", "0 0 * * 0"),
        ]

        for pattern, replacement in patterns:
            m = re.search(pattern, text, re.IGNORECASE)
            if m:
                groups = m.groups()
                if groups:
                    return replacement.format(*groups)
                return replacement

        return "*/15 * * * *"


class Heartbeat:
    RUN_ONCE_MARKER = "HEARTBEAT_COMPLETED"

    def __init__(self, config_path: str = None):
        self.config_path = config_path
        self.runner = TaskRunner()
        self.parser = ConfigParser()

        if config_path:
            self.config = self.parser.parse_file(config_path)
        else:
            self.config = {}

    def _check_run_once(self) -> bool:
        """Check if run_once_at is set and already ran. Returns True to skip."""
        run_once_at = self.config.get("run_once_at")
        if not run_once_at:
            return False

        try:
            target_time = datetime.fromisoformat(run_once_at.replace("Z", "+00:00"))
        except ValueError:
            try:
                target_time = datetime.strptime(run_once_at, "%Y-%m-%d %H:%M")
            except ValueError:
                logger.warning(f"Invalid run_once_at format: {run_once_at}")
                return False

        now = datetime.now()
        if now < target_time:
            logger.info(f"run_once_at not yet reached: {run_once_at}")
            return True

        if not self.config.get("log_file"):
            log_dir = pathlib.Path.home() / ".heartbeat" / "logs"
            job_name = self.config.get("name", "unknown")
            log_file = log_dir / f"{job_name}.log"
            self.config["log_file"] = str(log_file)

        log_file = self.config.get("log_file")
        if log_file and pathlib.Path(log_file).exists():
            marker = f"{self.RUN_ONCE_MARKER}_{self.config.get('name', '')}"
            try:
                content = pathlib.Path(log_file).read_text()
                if marker in content:
                    logger.info(f"run_once already completed, skipping")
                    return True
            except Exception:
                pass

        return False

    def _mark_run_once_complete(self):
        """Mark run_once as complete in log file."""
        if not self.config.get("log_file"):
            return

        marker = f"{self.RUN_ONCE_MARKER}_{self.config.get('name', '')}"
        try:
            with open(self.config["log_file"], "a") as f:
                f.write(f"\n{marker}\n")
        except Exception as e:
            logger.warning(f"Could not write run_once marker: {e}")

    def _send_notifications(self, success: bool):
        """Send notifications based on config."""
        notifications = self.config.get("notifications", [])
        if not notifications:
            return

        job_name = self.config.get("name", "unknown")
        status = "SUCCESS" if success else "FAILED"
        message = f"Heartbeat *{job_name}* {status}"

        for notif in notifications:
            notif_type = notif.get("type", "").lower()
            if notif_type == "telegram":
                on_failure = notif.get("on_failure", False)
                on_success = notif.get("on_success", False)

                should_send = (success and on_success) or (not success and on_failure)
                if not should_send:
                    continue

                plugin = get_plugin("telegram")
                if plugin:
                    plugin.send(notif, message)

    def run(self) -> bool:
        run_once_at = self.config.get("run_once_at")
        if run_once_at and self._check_run_once():
            print("SKIP_RUN_ONCE")
            return True

        folder = self.config.get("folder", ".")
        tasks = self.config.get("tasks", [])

        if not tasks:
            logger.warning("No tasks defined")
            return True

        context = {"folder": folder, "config": self.config}

        results = []
        for task in tasks:
            if not task:
                continue
            success = self.runner.run_task(task, context)
            results.append(success)

            if not success and task.get("on_fail"):
                on_fail = task["on_fail"]
                logger.info(f"Task failed, running on_fail: {on_fail[:50]}...")
                subprocess.run(on_fail, shell=True)

        all_passed = all(results) if results else True
        logger.info(f"Heartbeat completed: {'SUCCESS' if all_passed else 'FAILED'}")

        self._send_notifications(all_passed)

        if self.config.get("run_once_at"):
            self._mark_run_once_complete()
            print(f"REMOVE_CRON:{self.config.get('name', '')}")

        return all_passed


def main():
    parser = argparse.ArgumentParser(description="Heartbeat task runner")
    parser.add_argument("--job", "-j", help="Job name to run")
    parser.add_argument("--config", "-c", help="Config file path")
    parser.add_argument("--folder", "-f", help="Watch folder")
    parser.add_argument("--log", "-l", help="Log file path")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")

    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    if args.log:
        file_handler = logging.FileHandler(args.log)
        file_handler.setFormatter(
            logging.Formatter("%(asctime)s %(levelname)s %(message)s")
        )
        logger.addHandler(file_handler)

    config_path = args.config

    if not config_path and args.job:
        jobs_dir = pathlib.Path.home() / ".heartbeat" / "jobs"
        for ext in (".yaml", ".yml", ".htb"):
            candidate = jobs_dir / f"{args.job}{ext}"
            if candidate.exists():
                config_path = str(candidate)
                break

    if not config_path:
        logger.error("No config specified")
        sys.exit(1)

    hb = Heartbeat(config_path)
    success = hb.run()

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
