Update and licence controllers attempt all listener cleanup and discard stale
refreshes. Agent control matches DOM refs and label IDs literally. Bridge
validators use the generated domain ID rule and service failure codes.

## Breaking

- `NotificationController` state is held privately. Read projections through
  the existing getters, use `select()` and `dismissToast()` for local changes,
  and copy `records` or `toasts` before editing the list. Use constructor
  options for `pageSize`, toast selection, and action execution. Use
  `start()`, `stop()`, `observe()`, and `pendingCommands` in place of
  accessing the subscription, lifecycle, observers, or pending map. Construct
  a `NotificationClient` from the port for direct client calls. Toast IDs
  are opaque.
- Catch `BridgeOperationError` from `@inflatable-cookie/longhorn/operation/bridge`
  and branch on `outcome.kind`. Failed or cancelled terminals now throw
  that error from `acceptTerminal()`. An indeterminate mutation carries the
  host failure and must be reconciled before retrying.
- Handle `BridgeSupervisionError.code === "invalid_ownership"` for an invalid
  service ownership value. Transition errors still use `invalid_transition`.
