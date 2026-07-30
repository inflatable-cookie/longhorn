# 063 Transactional Linear Navigation And Failure Invariance

Status: complete
Owner: Tom
Roadmap: g01.011 batch 1
Governing refs: contracts 007, 008, and 010; research memo 015
Depends on: Card 062
Auto-start next card: no

## Objective

Implement one revision-bound plan/apply/commit protocol for undo, redo, and
entry-id checkout, including atomic compound and multi-entry failure behavior.

## Scope

- undo, redo, and entry-id checkout planning
- ordered inverse and forward payload batches
- exact source revision and target evidence
- consumer atomic apply transaction seam
- checked commit and transition receipt
- stale, failed, partial, rollback, and duplicate-commit handling
- bounded navigation depth

## Public Behavior

Planning does not move history. The consumer applies the complete ordered
batch atomically. Commit succeeds only against the source revision and exact
plan identity after product success.

Apply failure, rollback, stale commit, or duplicate commit leaves history
position and revision unchanged. Indexes are presentation values; stable entry
ids authorize checkout.

## Out Of Scope

- coalescing and gesture policy
- persistence
- renderer protocol
- branch checkout
- product transaction implementation

## Steps

1. Define navigation request, plan, target, and ordered step types.
2. Plan undo and redo without mutation.
3. Plan entry-id checkout in both directions.
4. Define the atomic consumer apply and rollback evidence seam.
5. Commit exact successful plans against current revision.
6. Return authoritative transition receipts and projection.
7. Reject stale, reused, oversized, and structurally corrupt plans.
8. Prove reverse compound order and complete failure invariance.
9. Re-run Loophole-shaped and non-editor transactions.

## Acceptance Criteria

- planning alone never changes state
- undo plans one inverse; redo plans one forward payload
- checkout plans the exact bounded ordered route
- stale and duplicate plans reject before state change
- apply failure preserves exact model, position, entries, and revision
- compounds and multi-entry checkout cannot partially commit
- rollback failure remains an explicit terminal and does not claim success
- Loophole-shaped successful navigation remains behaviorally equal

## Evidence Required

- plan and commit state-machine matrix
- forward/reverse ordering fixtures
- stale, duplicate, partial, and rollback failure injection
- exact pre/post model and history equality
- bounded navigation tests
- focused Rust and Effigy checks

## Stop Conditions

- the core must mutate the product model
- fallible payload application cannot be made atomic or rolled back
- stack position must move before product success
- renderer indexes are required as authority

## Next Task

Card 064 is ready. Add explicit coalescing, gesture groups, retention, and
authoritative projections over the transactional core.
