# Private History Tree Prototype

Card 068 evidence only. This crate is a nested, non-publishable workspace. It
is not a root workspace member, public Longhorn package, or Loophole
dependency.

## Frozen Questions

1. Can immutable single-parent nodes preserve abandoned futures without
   changing the public linear state?
2. Are derived leaf paths enough, or do stable names, annotations, pinning,
   and branch switching require first-class branch references?
3. Can mixed undo/redo checkout reuse the public typed step and rollback
   evidence while keeping its graph plan private?
4. Is preferred redo deterministic after record, divergence, and checkout?
5. Can count and encoded-weight pruning terminate while protecting current,
   named, and pinned lineage?
6. Can opaque checkpoints reduce measured replay cost without moving
   checkpoint content or durability into the graph?
7. Can a strict graph envelope migrate structure and consumer payloads
   independently and fail visibly?
8. Can the default projection remain one linear path while optional metadata
   exposes alternate paths?
9. Are depth, width, payload weight, checkout, encode/decode, and pruning
   costs credible for Loophole-shaped and small document workloads?

## Compared Models

Derived paths enumerate root-to-leaf routes. They are useful topology evidence
but have no stable identity when a leaf advances or disappears.

First-class branch references have injected stable ids and mutable
name/annotation/pin metadata outside immutable entry nodes. A reference head
can advance while its identity remains stable. The prototype implements both
views and records the tradeoff for Card 069.

## Fixture Matrix

| Shape | Depth | Width | Payload |
| --- | ---: | ---: | --- |
| document | 128 | 4 | small typed edits |
| Loophole-shaped | 2,048 | 64 alternate paths | weighted mutation-shaped bytes |

Exact measured values come from `measure:history-tree-prototype`. Durations are
evidence, not stable acceptance thresholds.

## Boundary

- public `longhorn-history` source and artifacts stay unchanged
- public `HistoryNavigationStep` and rollback evidence are reused
- the mixed LCA plan and transaction trait remain private here
- product apply, inverse meaning, payload codecs, checkpoints, and durability
  remain injected consumer authority
- no TypeScript, Tauri, Svelte, Poodle, project-version, collaboration, merge,
  event-source, or donor dependency enters the prototype
