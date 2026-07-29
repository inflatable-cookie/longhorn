# Registered Layout Persistence And Coordination

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 025
- added `longhorn-layout-config`
- bound one injected descriptor, default, registry, migration hook, and backup
  policy into a typed configuration domain
- added a canonical SHA-256 registry digest
- stored the digest beside the complete layout document inside the generic
  versioned configuration envelope
- made current-schema digest mismatch an exact preserved recovery state
- required schema bump plus explicit migration for registry changes
- applied Card 024 requests against fresh state inside store coordination
- returned layout and atomic publication receipts together
- restricted debounce to ordered sizing and collapse requests
- reused bounded weight, retained failure, explicit retry, and aggregate flush
- proved concurrent layout and window domains cannot replace each other

## Persistence Boundary

`longhorn-config` still owns roots, scope resolution, registration,
coordination, atomic replacement, recovery, backup, and debounce scheduling.
`longhorn-layout` still owns definitions, validation, normalization, and
mutation. The adapter translates between the two without importing Tauri,
Surface, TypeScript, Svelte, Poodle, or product state.

The outer configuration envelope carries domain id and schema version. Its raw
value carries the layout registry digest and complete document. Changing the
registry without changing schema fails as `InvalidValue` and preserves exact
source bytes. A migration hook can emit the new digest only while advancing
the registered schema.

## Coordination

Immediate mutation uses the existing store lock, fresh load, validation, and
atomic publication path. Card 024 rechecks the request revision inside that
critical section. Competing same-revision writers produce one publication and
one stale rejection.

Presentation debounce stages only `SetSizingSlot` and
`SetRegionCollapsed`. Ordered requests keep exact expected revisions and
publish one final complete document. Structural mutation remains immediate.
Lock timeout retains the same pending generation for explicit retry.

## Evidence

- 18 adapter contract tests pass
- missing, current, corrupt, future, migrated, missing-migration, and
  registry-mismatch loads are covered
- stale rejection preserves exact bytes
- two-store same-revision contention is covered
- structural-before-debounced ordering is covered
- bounded stage, helper-process timeout, retained generation, retry, and
  aggregate flush are covered
- explicit backup include/exclude policy is covered
- independent layout/window concurrency is covered
- Rust 1.85 config, layout, and adapter tests pass
- current warnings-denied Clippy passes
- Rust 1.85 workspace all-target check and full Effigy QA pass
- direct dependency graph contains core, config, layout, serde, and serde_json
  only

## Boundary

No storage scope, filename, project identity, window geometry codec, Tauri,
Surface, drag, renderer, TypeScript, Svelte, Poodle, product payload, or donor
write entered the adapter.

## Posture

`strict-ready`

Card 026 is the sole ready lane. Card 027 waits only on Card 026.

## Next

Review and explicitly start Card 026.
