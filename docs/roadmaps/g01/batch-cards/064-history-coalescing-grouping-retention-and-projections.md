# 064 History Coalescing, Grouping, Retention, And Projections

Status: complete
Owner: Tom
Roadmap: g01.011 batch 1
Governing refs: contracts 008 and 010; research memo 015
Depends on: Card 063
Auto-start next card: no

## Objective

Complete the public linear mechanics with deterministic coalescing, explicit
gesture groups, count and encoded-weight retention, and authoritative paged
past/current/future projections.

## Scope

- adjacent payload coalescing
- merge, no-op removal, and no-merge outcomes
- explicit group token lifecycle
- injected monotonic time and consumer duration
- entry-count and encoded-weight budgets
- exact pruning and retained-baseline receipts
- summary snapshot and bounded entry pages
- authoritative future entries

## Public Behavior

Coalescing runs only on compatible adjacent entries under consumer policy.
Gesture grouping uses explicit group identity and injected time. Navigation,
timeout, group close, authority replacement, and teardown end the group.

Pruning never invalidates the current product state. It advances the retained
baseline explicitly. Renderer projections report real future entries; clients
do not remember lost entries as authority.

## Out Of Scope

- structural persistence
- journal files
- TypeScript or UI implementation
- branch retention
- Loophole live gesture wiring

## Steps

1. Add checked adjacent coalesce transitions.
2. Add explicit group open, append, close, timeout, and cancellation.
3. Inject monotonic time and consumer grouping duration.
4. Add count and encoded-weight limits with overflow checks.
5. Define deterministic oldest-entry pruning and baseline evidence.
6. Project summary, next labels, current id, and bounded entry pages.
7. Expose past/current/future position and truncation.
8. Prove teardown and authority replacement close transient groups.
9. Validate Loophole 100-entry and 750 ms policy shapes without claiming live
   donor call sites.

## Acceptance Criteria

- coalescing cannot cross navigation or group boundaries
- a coalesced no-op removes the entry with an exact receipt
- no ambient clock enters the pure crate
- open groups do not persist or survive teardown
- count and encoded-weight limits are both enforced
- overflow and impossible budgets fail closed
- past, current, and future pages match the authoritative state
- Loophole can select its current limit and proposed gesture duration

## Evidence Required

- coalesce outcome matrix
- injected-time group lifecycle fixtures
- count, weight, overflow, and pruning matrix
- projection pagination and truncation fixtures
- no renderer-memory dependency proof
- focused Rust and Effigy checks

## Stop Conditions

- one fixed grouping duration must become library policy
- group identity depends on a product enum
- pruning can remove the current applied state without baseline evidence
- UI memory is required to reconstruct redo

## Next Task

Card 065 is ready. Add explicit structural persistence compatibility and the
committed transition seam used by consumer journals.
