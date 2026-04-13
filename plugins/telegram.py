import json
import urllib.request
import urllib.error
from typing import Optional

from .base import BasePlugin


class TelegramPlugin(BasePlugin):
    name: str = "telegram"

    def validate_config(self, config: dict) -> bool:
        api_token = self._get_env_var(config.get("api_token", ""))
        chat_id = self._get_env_var(config.get("chat_id", ""))
        return bool(api_token) and bool(chat_id)

    def send(self, config: dict, message: str, **kwargs) -> bool:
        api_token = self._get_env_var(config.get("api_token", ""))
        chat_id = self._get_env_var(config.get("chat_id", ""))

        if not api_token or not chat_id:
            return False

        url = f"https://api.telegram.org/bot{api_token}/sendMessage"
        payload = {"chat_id": chat_id, "text": message, "parse_mode": "Markdown"}

        try:
            data = json.dumps(payload).encode("utf-8")
            req = urllib.request.Request(
                url, data=data, headers={"Content-Type": "application/json"}
            )
            with urllib.request.urlopen(req, timeout=30) as resp:
                result = json.loads(resp.read().decode("utf-8"))
                return result.get("ok", False)
        except urllib.error.HTTPError as e:
            error_body = e.read().decode("utf-8")
            import logging

            logger = logging.getLogger("heartbeat")
            logger.error(f"Telegram HTTP error {e.code}: {error_body}")
            return False
        except Exception as e:
            import logging

            logger = logging.getLogger("heartbeat")
            logger.error(f"Telegram error: {e}")
            return False
