`NotificationController` state is held privately. Read projections through
the existing getters, use `select()` and `dismissToast()` for local changes,
and copy `records` or `toasts` before editing the list. Use constructor
options for `pageSize`, toast selection, and action execution. Use
`start()`, `stop()`, `observe()`, and `pendingCommands` in place of
accessing the subscription, lifecycle, observers, or pending map. Construct
a `NotificationClient` from the port for direct client calls. Toast IDs
are opaque.
