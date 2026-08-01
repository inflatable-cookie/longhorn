# longhorn-tauri-notifications

Narrow Tauri host assembly for an injected, caller-authorized notification
ledger. Register `TauriNotificationState`, expose the two command handlers, and
grant only the example capability needed by each window.

Committed mutations broadcast `longhorn://notifications/changed` to all app
windows. The event is an invalidation hint; renderers reload authority.
