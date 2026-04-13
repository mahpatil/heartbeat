from .base import BasePlugin


class SlackPlugin(BasePlugin):
    """Placeholder for Slack notification plugin."""

    name: str = "slack"

    def validate_config(self, config: dict) -> bool:
        return False

    def send(self, config: dict, message: str, **kwargs) -> bool:
        import logging

        logger = logging.getLogger("heartbeat")
        logger.warning("Slack plugin not implemented")
        return False
