from abc import ABC, abstractmethod
from typing import Any


class BasePlugin(ABC):
    name: str = "base"

    @abstractmethod
    def send(self, config: dict, message: str, **kwargs) -> bool:
        """Send notification. Returns True if successful."""
        pass

    def validate_config(self, config: dict) -> bool:
        """Validate plugin configuration. Returns True if valid."""
        return True

    def _get_env_var(self, value: str) -> str:
        """Resolve env:VAR_NAME to actual environment variable value."""
        if value.startswith("env:"):
            import os

            var_name = value[4:]
            return os.environ.get(var_name, "")
        return value
