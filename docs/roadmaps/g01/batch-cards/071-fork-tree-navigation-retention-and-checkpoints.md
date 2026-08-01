# 071 Fork-tree Navigation, Retention, And Checkpoints

Status: planned
Owner: Tom
Roadmap: g01.017 batch 1
Governing refs: contracts 001 and 008; Cards 063-064 and 068-070
Depends on: Card 070
Auto-start next card: yes

## Objective

Implement deterministic preferred redo, atomic LCA checkout, protected
retention, and opaque checkpoint accounting over the production graph.

## Scope

- revision-bound undo, redo, and branch checkout plans
- public typed step and rollback-evidence reuse
- preferred child updates only after successful commits
- current, named, and pinned lineage protection
- terminating count and exact-weight pruning
- opaque checkpoint refs and nearest-ancestor replay cost
- stale, apply, rollback, and impossible-budget failures

## Out Of Scope

- checkpoint snapshot content
- persistence encoding
- renderer protocols
- merge or collaboration

## Steps

1. Implement finite lineage and LCA planning.
2. Apply mixed routes through one consumer transaction.
3. Commit position and preferred edges atomically.
4. Implement protected-set retention and deterministic leaf pruning.
5. Add opaque checkpoint registration and replay accounting.
6. Run the full failure matrix at both fixture shapes.

## Acceptance Criteria

- preferred redo is deterministic
- checkout commits one complete route or no graph change
- verified rollback restores the exact model and graph
- rollback failure preserves graph and reports partial-model evidence
- protected pruning terminates or rejects without mutation
- checkpoint content stays outside Longhorn

## Evidence Required

- mixed-route fixtures
- failure-invariance matrix
- pruning and protection matrix
- checkpoint replay-cost fixtures
- bounded-depth evidence

## Stop Conditions

- checkout cannot reuse the atomic transaction seam
- retention can delete current or pinned lineage
- checkpoint content becomes graph authority

## Next Task

Card 072 adds dense persistence and independent migration.
