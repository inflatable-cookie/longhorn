# 111 Loophole Linear History Adoption

Status: active
Owner: Tom
Roadmap: g01.015 batch 4
Governing refs: contracts 003, 008, 010, 012, and 013; Card 110
Depends on: Card 110
Auto-start next card: no

## Objective

Replace Pulse's generic linear stack with Longhorn's structural history kernel
without losing one payload, inverse, grouping, cross-session undo, journal, or
recovery behavior.

## Repository Scope

- Longhorn: admitted history adapter fixes, fixtures, and artifact evidence.
- Loophole: Pulse payload codec/policy/apply, persistence, journal integration,
  Aura projection, tests, and docs.
- g01.017 fork-tree packages: unchanged and absent from the graph.

## Scope

- all 83 `PulseHistoryMutation` variants through a Pulse-owned typed adapter
- inverse, no-op, coalescing, explicit groups, automatic 750 ms grouping, limit 100
- full persisted applied/future ordering and exact import
- undo, redo, and entry-id checkout through plan/apply/commit
- atomic Pulse apply or verified rollback
- project-adjacent mutation/undo/redo journal integration
- authoritative paged past/future metadata and Poodle history panel
- visible incompatible/corrupt-history recovery; branch mode disabled

## Steps

1. Freeze every payload variant, label, inverse, coalesce, no-op, and apply match.
2. Register bounded Pulse codec and policy families with independent versions.
3. Convert `PulsePersistedHistorySnapshot` without replaying into canonical state.
4. Preserve entry ids, revisions, group boundaries, limits, and undo/redo position.
5. Route navigation through one Pulse product transaction and commit only after apply.
6. Drive the current session journal from committed structural receipts.
7. Preserve clean-save rotation, autosave suffix, checkpoint replay, and notifications.
8. Replace the eight-entry projection with authoritative bounded pages.
9. Remove only the old generic stack after prior-build and restart proof.

## Acceptance Criteria

- all 83 variants record, undo, redo, persist, reload, and journal through Pulse policy
- failed or stale apply changes neither model nor history revision/position
- complete undo and redo stacks survive cross-version migration
- valid checkpoint plus journal suffix recovers the same model and history
- corrupt or future history never silently becomes empty
- project version lineage remains independent
- no branch package or semantics enter the graph

## Evidence Required

- variant coverage and policy/apply parity report
- old/new envelope round trips and previous-build readback
- failure-invariance and multi-entry atomicity traces
- save/autosave/journal interruption matrix
- renderer metadata and capability audit

## Stop Conditions

- Pulse cannot provide atomic batch apply or verified rollback
- one live payload cannot round-trip through the registered codec
- compatibility requires silent empty-history fallback
- adoption depends on fork-tree work

## Next Task

Execute Card 112's full migration conformance and duplicate cleanup.
