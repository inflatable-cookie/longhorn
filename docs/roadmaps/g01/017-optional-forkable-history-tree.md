# g01.017 Optional Forkable History Tree

Status: executing; Cards 070-073 complete; Card 074 ready
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 008, 010, 012, and 013; Cards 068-069
Depends on: g01.016 linear consumer and release checkpoint

## Outcome

Implement the promoted fork-tree semantics as a separate optional production
layer without changing the public linear history contract or minimal consumer
graphs.

## Generation Runway

This lane follows real linear adoption. It turns the retained prototype into
new production packages; it does not retrofit branching into Loophole during
g01.015 or delay the first linear release.

## Execution Plan

### Batch 1: Pure graph authority

- [x] Card 070: immutable nodes, stable branch refs, divergence, and checked
  graph invariants
- [x] Card 071: preferred redo, atomic LCA navigation, protected retention,
  and opaque checkpoints

### Batch 2: Dense persistence

- [x] Card 072: strict graph envelope, dense payload representation,
  independent migration, and corruption proof

### Batch 3: Optional clients

- [x] Card 073: bounded metadata protocol, lazy alternate projections, Tauri,
  Svelte, and public-Poodle composition

### Batch 4: Artifact proof

- [ ] Card 074: isolated linear-only and tree-enabled installs, measured
  conformance, and production closeout

## Goals

- [x] Keep `longhorn-history` independent of tree state.
- [x] Preserve one payload copy per immutable node.
- [x] Make stable branch refs the only branch identity authority.
- [x] Reuse the proven consumer policy and atomic transaction seams.
- [x] Keep the ordinary projection linear and alternate data opt-in.
- [x] Bound every branch, path, persistence, navigation, and retention surface.
- [x] Keep project versions, collaboration, merge, and event sourcing out.

## Acceptance Criteria

- [x] Linear-only artifacts and dependency graphs remain unchanged.
- [x] Divergence, preferred redo, and LCA checkout match Card 068 semantics.
- [x] Failed and stale navigation preserves exact graph authority.
- [x] Protected pruning terminates under count and encoded-weight limits.
- [x] Checkpoint content and durability remain consumer-owned.
- [x] Dense persistence materially removes numeric-array expansion.
- [x] Alternate projections are lazy or paged and never duplicate unbounded
  lineage by default.
- [ ] Two isolated consumer shapes pass from produced artifacts.

## Explicit Non-goals

- changing linear history compatibility
- branch mode during g01.015 Loophole migration
- product versions, variants, merge, or collaboration
- generic product payloads in renderer protocols
- promoting the Card 068 prototype source directly

## Planning Checkpoint

Card 127 revalidates the working package names, coordinated private `0.1.0`,
Rust 1.85, TypeScript/Svelte/Tauri peers, exact-v1 protocols, and Loophole's
lossless linear consumer. Public registry names remain unresolved but do not
gate the private optional implementation. Cards 070-072 now supply the
downward-only graph, atomic navigation, protected retention, and opaque
checkpoint foundation plus dense strict persistence with document and
Loophole-shaped fixtures. Card 073 supplies the bounded payload-free protocol,
checked clients, narrow Tauri host, and optional Svelte/Poodle composition.
Card 074 retains artifact and closeout claims.

## Next Task

Execute Card 074. Prove isolated linear-only and tree-enabled artifacts and
close g01.017.
