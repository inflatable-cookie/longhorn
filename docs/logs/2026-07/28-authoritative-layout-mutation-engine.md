# Authoritative Layout Mutation Engine

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 024
- added bounded `LayoutRequestId`
- added strict request, command, outcome, receipt, and rejection envelopes
- implemented create, close, activate, reorder, move, sizing, and collapse
- admitted commands only at an exact expected revision
- committed one checked successor revision or no candidate state
- returned exact unchanged document evidence on every rejection
- added explicit bounded successful-request replay with conflict detection
- made Cards 025 and 026 ready

## Atomic Mutation

`LayoutMutationEngine` validates the supplied current document and expected
revision, clones one private candidate, applies one command, normalizes and
revalidates it, then returns the committed successor. The caller-owned source
is never mutated.

Create activates its new tab. Close and move select the member now at the
former index, then the previous final member. Reorder accepts only a complete
same-region permutation. Move rechecks registered policy and performs one
remove/insert across distinct regions or containers. Sizing and collapse use
registered slot and region policy.

## Failure And Replay

Typed rejections cover invalid source state, stale revision, overflow, unknown
identity, duplicate instance, placement and instance policy, close and move
policy, insertion, reorder membership, sizing, collapse, invalid candidate,
and request-id conflict. Each rejection carries the original revision and
document.

Ordinary `apply` has no replay behavior. `apply_with_replay` requires a caller
supplied `BoundedLayoutReplayStore`. Exact successful requests replay their
original receipt. Reusing an id for different content fails. Capacity is
explicit, finite, and oldest-success eviction is deterministic.

## Evidence

- 17 focused mutation tests pass
- all seven command success paths are covered
- all rejection fixtures compare unchanged serialized source bytes
- former-index active fallback covers first and final removal
- cross-region and cross-container move are covered
- singleton, one-per-container, bounded, and multiple instance policy pass
- structural input permutations produce one receipt
- request, command, receipt, and rejection serde are strict
- Loophole eight-region and Nucleus five-region mutation sequences use one
  engine
- Rust 1.85 package tests pass
- current-toolchain warnings-denied Clippy passes
- Rust 1.85 workspace all-target checks and full Effigy QA pass

## Boundary

No persistence, debounce, configuration, window, Surface, drag session, Tauri,
TypeScript, Svelte, Poodle, product payload, or donor write entered the
engine.

## Posture

`strict-ready`

Cards 025 and 026 are ready. Card 027 remains blocked on both.

## Next

Review and explicitly start Card 025. Card 026 is independently ready.
