from .base import BasePlugin


class SMSPlugin(BasePlugin):
    """Placeholder for SMS notification plugin."""

    name: str = "sms"

    def validate_config(self, config: dict) -> bool:
        return False

    def send(self, config: dict, message: str, **kwargs) -> bool:
        import logging

        logger = logging.getLogger("heartbeat")
        logger.warning("SMS plugin not implemented")
        return False
