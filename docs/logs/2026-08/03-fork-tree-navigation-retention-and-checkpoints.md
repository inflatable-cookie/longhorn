# Fork-tree Navigation, Retention, And Checkpoints

Date: 2026-08-03
Card: 071
Roadmap: g01.017

## Result

`longhorn-history-tree` now owns the complete in-memory production graph:

- history- and revision-bound undo, preferred redo, and branch checkout plans
- iterative lowest-common-ancestor routes with bounded typed steps
- one consumer-owned atomic apply transaction
- exact verified-rollback and rollback-failure evidence
- preferred-child updates only after committed navigation
- current, named, and pinned lineage protection
- deterministic oldest-leaf pruning by count and exact weight
- bounded opaque checkpoint refs and nearest-ancestor replay cost

## Atomic Navigation

The mixed-route fixture forks `B` into `C` and `D`, then checks out `C` from
`D` as one `undo D; redo C` batch. One consumer transaction call moves the
model from 7 to 6 and commits the graph only after success. Verified rollback
restores the exact model and graph. Rollback failure preserves graph authority
and reports explicit partial-model evidence. Stale plans make zero apply calls.

Redo prefers the last committed path. If several refs contain one shared
preferred child, the current branch wins; otherwise stable branch-id order is
the deterministic tie break.

## Retention And Checkpoints

Pruning protects every ancestor of the current branch head and current node,
plus every named or pinned branch head. It removes only unprotected leaves in
oldest sequence then stable-id order. Impossible protected budgets return
without mutation. Removing a preferred alternate restores preference to the
newest surviving direct child.

Checkpoints store an id, an optional retained node, and one opaque bounded
consumer reference. They contain no snapshot bytes or durability policy.
Replay cost selects the nearest ancestor checkpoint, with checkpoint-id order
as an equal-depth tie break. Pruning removes refs attached to deleted nodes and
reports them exactly.

## Evidence

- fifteen tree integration fixtures pass, including document and
  Loophole-shaped atomic failure paths
- mixed checkout, rollback, rollback failure, and stale-plan matrix passes
- anonymous pruning and named/pinned/current protection matrix passes
- checkpoint registration, replay, invalid import, and stale-operation matrix
  passes
- a 2,048-node lineage plans iteratively with 2,048 exact redo steps
- focused formatting, Clippy, package, dependency, and API-reference QA passes

Loophole and every linear package remain unchanged. Product apply, checkpoint
content, storage, project versions, merge, and collaboration remain consumer
authority.

## Next Task

Execute Card 072. Add strict dense persistence, independent structural and
payload migration, and corruption proof.
