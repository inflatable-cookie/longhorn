# 087 Native-content Identity, State, And Planning

Status: complete
Owner: Tom
Roadmap: g01.018 batch 1
Governing refs: contracts 001, 003, 009, 012, and 017; Card 086
Depends on: Card 086
Auto-start next card: no

## Objective

Implement the production pure native-content kernel from the promoted
contract. Preserve the lossless three-shape semantics without treating the
Card 082 prototype as package authority.

## Scope

- `longhorn-native-content` workspace crate
- bounded island and kind identity
- host binding, attach generation, and revisions
- mechanism descriptors and capability validation
- typed desired and observed coordination state
- client viewport, explicit scale, and rounding
- ordered update, attach, and detach plans
- exact apply, failure, skipped, and teardown receipts
- stale observation, proposal, and completion rejection
- child-view, isolated-window, and backing-surface fixtures

## Out Of Scope

- Tauri or native APIs
- TypeScript, Svelte, or Poodle
- browser, plugin, process, GPU, renderer, or semantic-input implementation
- outer-window placement
- donor repository changes
- copying the prototype crate into the root workspace unchanged

## Steps

1. Add the production crate and workspace metadata.
2. implement bounded identities, generations, revisions, and descriptors.
3. Implement typed geometry, scale validation, and mechanism mapping.
4. Implement desired/observed state and pure operation planning.
5. Implement proposals, partial receipts, convergence, and teardown evidence.
6. Freeze product-neutral three-shape fixtures from prototype outcomes.
7. Prove stale and failure invariance plus dependency isolation.
8. Document public API and differences from the retained prototype.

## Acceptance Criteria

- one public vocabulary represents all three mechanisms losslessly
- viewport effect stays mechanism-specific
- invalid scale, geometry, revision, or generation cannot mutate state
- partial apply lists attempted, failed, and dependency-skipped operations
- fresh observation decides convergence
- detach and host destroy are explicit and idempotent at the coordination seam
- no raw handle or product payload enters public types
- normal dependencies contain only approved pure workspace foundations

## Evidence Required

- three-shape trace matrix
- deterministic 1x/2x geometry fixtures
- stale generation/revision and partial-failure fixtures
- lifecycle and teardown matrix
- public API and dependency audit
- focused Rust, clippy, docs, format, and Effigy checks

## Stop Conditions

- production semantics require a mechanism-specific product payload
- the three mechanisms require incompatible common state
- outer-window placement or content lifecycle enters the kernel
- exact receipt semantics cannot represent one retained packaged trace

## Next Task

Execute ready Card 088. Generate the checked protocol from the final Rust
surface and add the framework-neutral client without starting native adapters.

## Completion Evidence

- Added production `longhorn-native-content` and shared core ids/revision.
- Kept normal dependencies to `longhorn-core` and `serde`.
- Proved child bounds, isolated content size, and backing viewport clip through
  one public vocabulary.
- Added nonzero generation/step decoding, capability-bound input, host-bound
  generations, explicit host invalidation, and non-repeating detach plans.
- Bound partial receipts to exact island, desired revision, observed revision,
  generation, operations, failures, and dependency skips.
- Added 17 contract tests covering three shapes, geometry, atomic rejection,
  lifecycle, teardown, proposals, receipts, and dependency boundaries.
- Recorded the production API and prototype differences in the crate README
  and canonical composition guide.
