# 075 Operation Donor Fixtures, Identity, And Lifecycle Authority

Status: complete
Owner: Tom
Roadmap: g01.012 batch 1
Governing refs: contracts 001, 003, 012, and 015; research memo 016
Depends on: Card 069
Auto-start next card: no
Completed: 2026-07-31

## Objective

Freeze the strong Soundcheck and Loophole operation shapes as
consumer-neutral fixtures. Implement the pure bounded identity, catalogue,
state, and terminal-transition authority.

## Scope

- `longhorn-operation` crate
- operation, kind, scope, phase, and authority identity
- operation and catalogue revisions
- queued, running, cancelling, succeeded, failed, cancelled, interrupted
- consumer-decided queued or direct-running registration
- checked forward transitions and sticky terminals
- active and recent projections without final retention policy
- Soundcheck scan and Loophole render fixtures
- weaker Nucleus, Split-shell, and Jetstream boundary notes

## Public Behavior

Consumers register a stable operation in `queued` or `running`. Longhorn
checks identity, metadata bounds, expected catalogue revision, and lifecycle
transitions. The consumer supplies every executor fact.

Terminal state is immutable. `cancelling` may still end in success or failure.
No queue order, same-active reuse, work execution, or product result enters the
crate.

## Out Of Scope

- progress semantics beyond a placeholder revision seam
- cancellation command and executor token
- retention, teardown, and recovery
- serialization, TypeScript, Tauri, bridge, Svelte, or Poodle
- notification records
- donor repository writes

## Steps

1. Add the pure crate and bounded identity/revision types.
2. Define the closed lifecycle and legal transition table.
3. Add consumer-decided queued and direct-running registration.
4. Implement expected-revision state transitions and terminal stickiness.
5. Add bounded metadata and catalogue sequence validation.
6. Add active and recent projections without silent eviction.
7. Freeze Soundcheck scan and Loophole render lifecycle fixtures.
8. Record which Nucleus states and donor fields remain product workflow.
9. Audit dependencies and public product-payload leakage.

## Acceptance Criteria

- both strong donors use one public transition API
- queued and direct-running registration pass
- all illegal edges reject with exact unchanged state
- success and failure are legal after cancellation request state
- every terminal is sticky
- operation and catalogue revisions are distinct and monotonic
- identity does not depend on time, randomness, bridge request, or renderer
- no queue, executor, arbitrary JSON, or product enum enters the crate
- pure graph imports no async runtime, bridge, config, Tauri, Svelte, or Poodle

## Evidence Required

- complete transition matrix
- Soundcheck scan fixture
- Loophole render fixture
- illegal-edge and duplicate-terminal fixtures
- id, revision, metadata-bound, overflow, and ordering fixtures
- donor retained/rejected matrix
- dependency and public-API audit
- focused Rust, clippy, docs, formatting, and Effigy checks

## Stop Conditions

- one donor requires queue scheduling in the shared authority
- success after cancellation acceptance cannot be represented truthfully
- product outcome payload must enter the generic state
- operation identity must become a transport request id
- a third lifecycle is required to represent the two strong donors

## Next Task

Execute ready Card 076. Add bounded progress, cancellation receipts,
retention, and controlled teardown.
