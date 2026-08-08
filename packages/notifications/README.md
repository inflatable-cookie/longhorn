# @inflatable-cookie/longhorn-notifications

Checked framework-neutral client for the finite retained notification ledger.
The root exports strict generated types, compatibility guards, listener-first
reconciliation, direct/serialized ports, paging, seen/dismiss commands, and a
presentation controller.

Optional subpaths:

- `/tauri`: invoke/event transport composition
- `/svelte`: one isolated rune session per renderer instance
- `/poodle`: controlled retained panel plus public `ToastHost` and `ToastStack`

Semantic actions remain data. Inject `NotificationActionExecutor`; it performs
fresh consumer admission on every invocation. Toast expiry only removes the
local transient projection. It never marks a retained record seen or dismissed.
