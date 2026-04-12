#!/usr/bin/env python3
"""Heartbeat - scheduled task runner with config as code"""

import argparse
import sys
import subprocess
import time
import logging
import pathlib
import re
import urllib.request
import urllib.error
import os

try:
    import yaml

    YAML_AVAILABLE = True
except ImportError:
    YAML_AVAILABLE = False


def parse_simple_yaml(content: str) -> dict:
    """Simple YAML parser for basic configs"""
    result = {"tasks": []}
    tasks = result["tasks"]
    current_task = None
    indent_level = 0

    for line in content.split("\n"):
        original = line
        stripped = line.strip()

        if not stripped or stripped.startswith("#"):
            continue

        indent = len(original) - len(original.lstrip())

        if stripped.startswith("-"):
            indent = 0
            stripped = stripped.lstrip("- ").strip()

        if ":" not in stripped:
            continue

        key, _, value = stripped.partition(":")
        key = key.strip()
        value = value.strip().strip('"')

        if key in ("name", "folder", "frequency"):
            result[key] = value

        elif key == "type":
            current_task = {"type": value}
            tasks.append(current_task)

        elif key == "agent":
            current_task = {"agent": value}
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
            "params",
            "args",
        ):
            if current_task:
                current_task[key] = value

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

    def _run_agent(self, task: dict, ctx: dict) -> bool:
        agent = task.get("agent") or task.get("name") or task.get("type")
        prompt = task.get("prompt", "")
        folder = ctx.get("folder", ".")

        params = task.get("params") or task.get("args") or []
        if isinstance(params, str):
            params = params.split()

        if not agent or not prompt:
            logger.error("Agent task missing agent name or prompt")
            return False

        full_prompt = f"In {folder}: {prompt}"

        cmd = [agent]
        cmd.extend(params)
        cmd.append(full_prompt)
        logger.info(f"Running agent: {agent} {' '.join(params)} [prompt]")

        try:
            result = subprocess.run(cmd, capture_output=True, timeout=600)
            if result.returncode == 0:
                logger.info(f"Agent {agent} succeeded")
                return True
            else:
                logger.error(f"Agent {agent} failed: {result.stderr.decode()[:200]}")
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

        config = {
            "name": name_match.group(1).strip() if name_match else "Unnamed",
            "folder": folder_match.group(1).strip() if folder_match else ".",
            "frequency": self._parse_frequency(freq_match.group(1))
            if freq_match
            else "*/15 * * * *",
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
            elif key in ("params", "args"):
                if current_task:
                    current_task["params"] = (
                        value  # Keep as string, split in _run_agent
                    )
            elif key == "resume":
                if current_task:
                    current_task.setdefault("params", []).extend(["--resume", value])
            elif key == "skip permissions":
                if current_task:
                    current_task.setdefault("params", []).append(
                        "--dangerously-skip-permissions"
                    )
            elif key == "with":
                if current_task:
                    current_task["params"] = value.split()
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
            (r"daily\s+at\s+(\d{1,2}):(\d{2})", None),
            (r"daily", "0 * * * *"),
            (r"weekly\s+on\s+monday", "0 0 * * 1"),
            (r"weekly", "0 0 * * 0"),
        ]

        for pattern, replacement in patterns:
            m = re.search(pattern, text, re.IGNORECASE)
            if m:
                if pattern == r"daily\s+at\s+(\d{1,2}):(\d{2})" and m.groups():
                    hour = int(m.group(1))
                    return f"0 {hour} * * *"
                groups = m.groups()
                if groups and replacement:
                    return replacement.format(*groups)
                elif replacement:
                    return replacement

        return "*/15 * * * *"


class Heartbeat:
    def __init__(self, config_path: str = None):
        self.config_path = config_path
        self.runner = TaskRunner()
        self.parser = ConfigParser()

        if config_path:
            self.config = self.parser.parse_file(config_path)
        else:
            self.config = {}

    def run(self) -> bool:
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
