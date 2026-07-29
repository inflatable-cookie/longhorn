# g01.005 Layout Container, Region, And Panel Core

Status: complete
Owner: Tom
Updated: 2026-07-28
Governing refs: contracts 001, 002, 004, 010, 012, and 014; research memo 009

## Outcome

Ship the Surface-independent Rust layout state machine, registered persistence
adapter, and checked TypeScript protocol shared by Loophole and Nucleus.

## Generation Runway Goal

Advance the shared desktop foundation from native window hosting into
composable workspace state. Preserve the direct-window and optional-Surface
shapes needed by `g01.006`, `g01.014`, and `g01.015`.

## Goals

- [x] Add bounded layout identity, definitions, snapshots, and visibility
  projection.
- [x] Add expected-revision create, close, activate, reorder, move, sizing, and
  collapse mutation.
- [x] Persist layout through an injected registered configuration domain.
- [x] Generate checked TypeScript snapshots, commands, receipts, and errors.
- [x] Prove Loophole eight-region and Nucleus five-region compositions through
  one engine.
- [x] Keep Surface, product payload, Tauri, Svelte, and Poodle dependencies out
  of the core.

## Execution Plan

### Batch 1: Model and policy

- [x] [Card 023](batch-cards/023-layout-identity-policy-and-normalization.md) —
  layout identity, registered schemas, panel policy, normalized snapshots, and
  derived visibility.

### Batch 2: State and mutation

- [x] [Card 024](batch-cards/024-authoritative-layout-mutation-engine.md) —
  atomic expected-revision structural and sizing mutation.
- [x] [Card 025](batch-cards/025-registered-layout-persistence-and-coordination.md)
  — registered configuration adapter, fresh-state coordination, debounce, and
  flush.

### Batch 3: Protocol and conformance

- [x] [Card 026](batch-cards/026-generated-layout-typescript-protocol.md) —
  checked Rust-to-TypeScript protocol artifacts and framework-neutral helpers.
- [x] [Card 027](batch-cards/027-two-shape-layout-conformance-and-closeout.md) —
  Loophole/Nucleus conformance, cross-language fixtures, and milestone
  closeout.

## Acceptance Criteria

- [x] Both donor fixtures use the same registry, resolver, and mutation engine.
- [x] Nucleus uses direct window binding without importing Surface state.
- [x] Loophole adapts Surface identity only outside `longhorn-layout`.
- [x] Missing placement or instance policy fails closed.
- [x] Invalid or stale mutation preserves the exact durable document.
- [x] Active selection, ordering, collapse, sizing, and empty visibility are
  deterministic.
- [x] Layout persistence cannot overwrite host-owned window geometry.
- [x] Rust and generated TypeScript fixtures round-trip without handwritten
  duplicate DTOs.
- [x] Rust 1.85 and full Effigy QA pass.

## Card Runway

| Card | State | Unlocks |
| --- | --- | --- |
| 023 | complete | pure layout model and registry |
| 024 | complete | authoritative mutation |
| 025 | complete | durable coordinated state |
| 026 | complete | checked renderer protocol |
| 027 | complete | two-shape proof and closeout |

## Boundaries

- No arbitrary recursive split tree.
- No Surface lifecycle or hosting preference.
- No cross-window transfer session or drop-zone lease.
- No Svelte store, Poodle binding, Tauri handler, or packaged UI.
- No Loophole or Nucleus repository writes.
- No product panel title, body, resource attachment, or runtime handle.

## Planning Checkpoint

The library milestone is closed without claiming donor ownership transfer.
`g01.006` is now compiled against the implemented layout protocol and revised
contract 011. Consumer cutover remains `g01.014` onward.

## Next Task

Card 028 is ready under the compiled `g01.006` runway. Start it from its own
card, not this completed milestone.
