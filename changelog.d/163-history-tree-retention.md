### Breaking

- Fork history metadata protocol is version 2. Upgrade host authorities and clients together. Delete, including `ForkHistorySession.deleteContinuation`, returns `ForkDeleteResult` with `deleted` or `rejected`; prune can return `rejected`. Match those results and refresh when a rejection requests it.
- Fork record rejects no-op payloads and group metadata with `NoOpPayload` or `GroupRequiresLifecycle`. Skip no-ops before recording and keep grouped history in an authority with a group lifecycle.
- `ForkHistoryError` has two new variants. Update exhaustive Rust matches.

### Fixed

- Pruning a branch tip retains its surviving continuation and branch reference.
- Checking out a different branch at a shared node commits a stationary navigation.

- Repair a stale native-content incompatibility fixture import on main; its five cases continue to check the v2 fixture.
