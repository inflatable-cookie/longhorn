# Operation Donor Fixtures, Identity, And Lifecycle Authority

Date: 2026-07-31
Card: 075
Roadmap: g01.012

## Result

Implemented the first pure `longhorn-operation` slice. It provides bounded
operation, kind, scope, phase, and authority identities; a nonzero authority
epoch; distinct operation and catalogue revisions; finite catalogue
registration; exact lifecycle transitions; sticky terminals; and active and
recent projections.

Registration remains consumer-admitted. Soundcheck-shaped work can register
directly as running. Loophole-shaped work can register queued and later start.
The crate neither schedules nor executes either shape.

## Donor Disposition

| Evidence | Retained | Consumer-owned or rejected |
| --- | --- | --- |
| Soundcheck scan | stable identity, direct-running registration, remount-safe host truth, cancellation race terminal | same-active reuse, scan coordinator, phases, plugin counts, warnings, reconciliation |
| Loophole render | queued registration, explicit start, insertion-ordered active view, newest-terminal view | queue order, pause, executor, polling, report, artifact, cleanup, export policy |
| Nucleus turn | generic cancelling and terminal state vocabulary | approval, input, tools, tokens, provider state, recovery workflow |
| Bovine and Jetstream busy state | evidence for later small projections | Git and engine task meaning |

No donor repository changed.

## Lifecycle Authority

The closed states are `queued`, `running`, `cancelling`, `succeeded`,
`failed`, `cancelled`, and `interrupted`. The public transition table is exact.
Terminal states cannot reopen. A cancelling operation may still succeed or fail
when executor completion wins the race.

Every mutation checks the authority cursor and expected revision before
commit. Failed stale, foreign, duplicate, unknown, invalid-edge, limit, or
overflow attempts leave the exact catalogue unchanged.

## Boundary Audit

The pure crate depends only on `longhorn-core`. It contains no async runtime,
executor, scheduler, config, persistence, bridge, Tauri, TypeScript, Svelte,
Poodle, arbitrary JSON, or product outcome enum. Operation identity is not a
transport request id, timestamp, random value, or renderer key.

Progress, cancellation commands, retention, retry lineage, controlled
teardown, serialization, transports, and presentation remain outside this
card.

## Validation

- `effigy test:operation-core` passed: 41 unit, integration, and doc tests
- strict clippy passed for `longhorn-core` and `longhorn-operation`
- `effigy qa:northstar` and the Card 075 path selector passed
- complete registration and 7-by-7 lifecycle matrices pass
- Soundcheck, Loophole, failure-invariance, ordering, bound, and overflow
  fixtures pass
- dependency audit confirms `longhorn-operation` has one direct dependency:
  `longhorn-core`

The closeout-wide `effigy qa` passed its Rust workspace, TypeScript, Svelte,
Poodle, and earlier artifact stages, then one temporary Loophole history
consumer exited from Vitest without an assertion result. Immediate isolated
rerun of `effigy proof:history-system-artifacts` passed with the recorded
history and Poodle hashes unchanged. This non-reproducing existing-proof
failure is outside Card 075; the focused operation and Northstar gates remain
clean.

## Next Task

Execute Card 076. Add bounded progress, receipted cancellation, exact
retention, retry lineage, and controlled teardown without importing executor
or product authority.
