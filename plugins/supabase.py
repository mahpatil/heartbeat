"""
Supabase notification plugin.

PLANNED - Not yet implemented.

This plugin will support two modes:

1. Query pending actions:
   - Connect to Supabase table `pending_notifications`
   - Query rows where `status = 'pending'`
   - Send each as notification via other plugins (telegram, etc.)
   - Update row to `sent` with timestamp

2. Track sends:
   - Insert into `notification_log` table after each send
   - Fields: id, notification_type, recipient, message, sent_at, status

Example config:
```yaml
notifications:
  - type: supabase
    url: https://xxx.supabase.co
    key: env:SUPABASE_KEY
    table: pending_notifications
    query: "status=pending"
    on_failure: true
```

Requirements:
- python: ``pip install supabase``
- Environment: SUPABASE_KEY or similar
"""

from .base import BasePlugin


class SupabasePlugin(BasePlugin):
    """Placeholder for Supabase notification plugin."""

    name: str = "supabase"

    def validate_config(self, config: dict) -> bool:
        return False

    def send(self, config: dict, message: str, **kwargs) -> bool:
        import logging

        logger = logging.getLogger("heartbeat")
        logger.warning("Supabase plugin not implemented")
        return False
