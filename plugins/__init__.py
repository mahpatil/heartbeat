from .base import BasePlugin
from .telegram import TelegramPlugin

PLUGINS = {
    "telegram": TelegramPlugin,
}


def get_plugin(name: str) -> BasePlugin:
    """Get plugin instance by name. Returns None if not found."""
    cls = PLUGINS.get(name.lower())
    if cls is None:
        return None
    return cls()


def list_plugins() -> list[str]:
    """List available plugin names."""
    return list(PLUGINS.keys())
