#!/usr/bin/env python3
"""Tests for heartbeat"""

import os
import sys
import tempfile
import pathlib
from types import SimpleNamespace
from unittest.mock import patch

sys.path.insert(0, str(pathlib.Path(__file__).parent.parent))

from heartbeat import parse_simple_yaml, ConfigParser, TaskRunner, Heartbeat


class TestParseSimpleYaml:
    """Test YAML config parser"""

    def test_parse_name_folder_frequency(self):
        """Test parsing basic fields"""
        content = """name: test-job
folder: ~/project
frequency: */15 * * * *"""
        result = parse_simple_yaml(content)
        assert result["name"] == "test-job"
        assert result["folder"] == "~/project"
        assert result["frequency"] == "*/15 * * * *"

    def test_parse_run_task(self):
        """Test parsing run task"""
        content = """name: test
tasks:
- type: run
  command: echo hello"""
        result = parse_simple_yaml(content)
        assert len(result["tasks"]) == 1
        assert result["tasks"][0]["type"] == "run"
        assert result["tasks"][0]["command"] == "echo hello"

    def test_parse_url_task(self):
        """Test parsing URL task"""
        content = """name: test
tasks:
- type: url
  url: https://example.com
  on_fail: echo fail"""
        result = parse_simple_yaml(content)
        assert len(result["tasks"]) == 1
        assert result["tasks"][0]["type"] == "url"
        assert result["tasks"][0]["url"] == "https://example.com"
        assert result["tasks"][0]["on_fail"] == "echo fail"

    def test_parse_file_exists_task(self):
        """Test parsing file_exists task"""
        content = """name: test
tasks:
- type: file_exists
  path: data.json"""
        result = parse_simple_yaml(content)
        assert len(result["tasks"]) == 1
        assert result["tasks"][0]["type"] == "file_exists"
        assert result["tasks"][0]["path"] == "data.json"

    def test_parse_agent_task(self):
        """Test parsing agent task"""
        content = """name: test
tasks:
- agent: claude
  prompt: Review code"""
        result = parse_simple_yaml(content)
        assert len(result["tasks"]) == 1
        assert result["tasks"][0]["agent"] == "claude"
        assert result["tasks"][0]["prompt"] == "Review code"

    def test_parse_agent_with_params(self):
        """Test parsing agent with params"""
        content = """name: test
tasks:
- agent: claude
  params: --resume mysession --dangerously-skip-permissions
  prompt: Run ls"""
        result = parse_simple_yaml(content)
        assert len(result["tasks"]) == 1
        assert result["tasks"][0]["agent"] == "claude"
        assert (
            result["tasks"][0]["params"]
            == "--resume mysession --dangerously-skip-permissions"
        )
        assert result["tasks"][0]["prompt"] == "Run ls"

    def test_parse_multiple_tasks(self):
        """Test parsing multiple tasks"""
        content = """name: test
tasks:
- type: run
  command: echo step1

- type: url
  url: https://example.com"""
        result = parse_simple_yaml(content)
        assert len(result["tasks"]) == 2
        assert result["tasks"][0]["type"] == "run"
        assert result["tasks"][1]["type"] == "url"

    def test_skip_comments(self):
        """Test skipping comment lines"""
        content = """name: test
# This is a comment
tasks:
- type: run
  command: echo hello"""
        result = parse_simple_yaml(content)
        assert result["name"] == "test"
        assert result["tasks"][0]["command"] == "echo hello"


class TestConfigParser:
    """Test ConfigParser class"""

    def test_parse_yaml_file(self):
        """Test parsing YAML file"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("""name: test
folder: .
frequency: */15 * * * *
tasks:
- type: run
  command: echo hello""")
            f.flush()
            try:
                parser = ConfigParser()
                result = parser.parse_file(f.name)
                assert result["name"] == "test"
                assert result["tasks"][0]["command"] == "echo hello"
            finally:
                os.unlink(f.name)

    def test_parse_nl_file(self):
        """Test parsing natural language file"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".htb", delete=False) as f:
            f.write("""# Heartbeat: My check
# Folder: /tmp

Every 15 minutes:
  - Run: echo hello
  - URL reachable: https://example.com""")
            f.flush()
            try:
                parser = ConfigParser()
                result = parser.parse_file(f.name)
                assert result["name"] == "My check"
                assert result["folder"] == "/tmp"
                assert len(result["tasks"]) == 2
            finally:
                os.unlink(f.name)


class TestTaskRunner:
    """Test TaskRunner class"""

    def test_run_shell_success(self):
        """Test running successful shell command"""
        runner = TaskRunner()
        result = runner._run_shell("echo hello", {})
        assert result is True

    def test_run_shell_failure(self):
        """Test running failing shell command"""
        runner = TaskRunner()
        result = runner._run_shell("exit 1", {})
        assert result is False

    def test_check_url_success(self):
        """Test checking reachable URL"""
        runner = TaskRunner()
        result = runner._check_url("https://example.com", {})
        assert result is True

    def test_check_url_failure(self):
        """Test checking unreachable URL"""
        runner = TaskRunner()
        result = runner._check_url("https://example.invalid", {})
        assert result is False

    def test_check_file_exists(self):
        """Test checking existing file"""
        runner = TaskRunner()
        result = runner._check_file(__file__, {})
        assert result is True

    def test_check_file_not_exists(self):
        """Test checking non-existent file"""
        runner = TaskRunner()
        result = runner._check_file("/nonexistent/file.txt", {})
        assert result is False

    def test_run_agent_uses_shell_runner(self):
        """Test agent tasks are delegated to shell runner script"""
        runner = TaskRunner()
        task = {"agent": "opencode", "prompt": "Review code"}

        fake_result = SimpleNamespace(returncode=0, stdout=b"ok", stderr=b"")
        with patch("heartbeat.subprocess.run", return_value=fake_result) as mock_run:
            result = runner._run_agent(task, {"folder": "."})

        assert result is True
        cmd = mock_run.call_args[0][0]
        assert cmd[0].endswith("heartbeat-agent-runner.sh")
        assert cmd[1] == "opencode"
        assert cmd[2] == "."
        assert cmd[3] == "Review code"


class TestHeartbeatClass:
    """Test Heartbeat class"""

    def test_run_empty_config(self):
        """Test running empty config"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("name: test\ntasks: []")
            f.flush()
            try:
                hb = Heartbeat(f.name)
                result = hb.run()
                assert result is True
            finally:
                os.unlink(f.name)

    def test_run_url_task(self):
        """Test running URL task"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("""name: test
tasks:
- type: url
  url: https://example.com""")
            f.flush()
            try:
                hb = Heartbeat(f.name)
                result = hb.run()
                assert result is True
            finally:
                os.unlink(f.name)

    def test_run_shell_task(self):
        """Test running shell task"""
        with tempfile.NamedTemporaryFile(mode="w", suffix=".yaml", delete=False) as f:
            f.write("""name: test
tasks:
- type: run
  command: echo hello""")
            f.flush()
            try:
                hb = Heartbeat(f.name)
                result = hb.run()
                assert result is True
            finally:
                os.unlink(f.name)


class TestFrequencyParser:
    """Test frequency parsing"""

    def test_parse_frequency(self):
        """Test parsing various frequencies"""
        parser = ConfigParser()

        assert parser._parse_frequency("every 15 min") == "*/15 * * * *"
        assert parser._parse_frequency("every 30 min") == "*/30 * * * *"
        assert parser._parse_frequency("every hour") == "0 * * * *"
        assert parser._parse_frequency("hourly") == "0 * * * *"
        assert parser._parse_frequency("daily") == "0 * * * *"
        assert parser._parse_frequency("daily at 9:00") == "0 9 * * *"
        assert parser._parse_frequency("weekly on monday") == "0 0 * * 1"


if __name__ == "__main__":
    import traceback

    test_classes = [
        TestParseSimpleYaml,
        TestConfigParser,
        TestTaskRunner,
        TestHeartbeatClass,
        TestFrequencyParser,
    ]

    passed = 0
    failed = 0

    for cls in test_classes:
        print(f"\n{cls.__name__}:")
        instance = cls()
        for method_name in dir(instance):
            if method_name.startswith("test_"):
                try:
                    getattr(instance, method_name)()
                    print(f"  PASS: {method_name}")
                    passed += 1
                except Exception as e:
                    print(f"  FAIL: {method_name} - {e}")
                    failed += 1

    print(f"\n{'=' * 40}")
    print(f"Results: {passed} passed, {failed} failed")
