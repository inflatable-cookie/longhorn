# longhorn-operation

Pure finite asynchronous-operation lifecycle authority. Consumers own work
admission, queue scheduling, execution, product progress, reports, artifacts,
authorization, persistence, and recovery.

Cards 075-076 supply:

- bounded operation, kind, scope, phase, and authority ids
- nonzero authority epoch
- distinct operation revision, catalogue revision, and insertion sequence
- queued, running, cancelling, succeeded, failed, cancelled, and interrupted
- consumer-decided queued or direct-running registration
- checked legal transitions and immutable terminal states
- active insertion-order and recent newest-first projections
- monotonic indeterminate, unit, normalized, and phase progress
- cancellation support, revision-bound admission, and race-safe receipts
- separate active, terminal-count, and terminal encoded-weight limits
- exact oldest-first eviction and explicit terminal dismissal receipts
- terminal-only retry lineage through a new operation id
- complete atomic teardown through terminal or transfer outcomes
- Soundcheck plugin-scan and Loophole render-queue fixtures

Serialization, TypeScript, Tauri, bridge, Svelte, Poodle, and notifications
remain later cards.

## Donor boundary

| Donor behavior | Shared in Card 075 | Consumer-owned |
| --- | --- | --- |
| Soundcheck stable scan job | id, direct-running registration, lifecycle | same-active reuse and scan coordinator |
| Soundcheck phases and counts | phase id type only | scan phases, plugin units, warnings, reconciliation |
| Loophole queued render | queued registration and start transition | queue order, pause, executor, polling |
| Loophole terminal render | sticky generic outcome | report, artifact, cleanup, export policy |
| Nucleus agent turn | generic cancellation and terminal shape only | approval, input, tool, token, provider, recovery workflow |
| Split-shell and Jetstream busy state | future small projection fit | Git and engine task meaning |

The crate contains no executor trait or arbitrary product payload. Bridge
request ids do not identify catalogue operations.
