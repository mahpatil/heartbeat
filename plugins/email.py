from .base import BasePlugin


class EmailPlugin(BasePlugin):
    """Placeholder for email notification plugin."""

    name: str = "email"

    def validate_config(self, config: dict) -> bool:
        return False

    def send(self, config: dict, message: str, **kwargs) -> bool:
        import logging

        logger = logging.getLogger("heartbeat")
        logger.warning("Email plugin not implemented")
        return False
