# 026 Generated Layout TypeScript Protocol

Status: complete
Owner: Tom
Roadmap: g01.005 batch 3
Governing refs: contracts 001, 010, 012, 013, and 014; research memos 003 and 009
Auto-start next card: no

## Objective

Generate and package the layout snapshot, command, receipt, and error protocol
from Rust authority without starting the Svelte/Poodle adapter lane.

## Scope

- usable `longhorn-bindings` generation slice for the layout domain
- checked generated files in `@longhorn/layout`
- snapshots, definitions, mutations, receipts, and typed errors
- compatibility/version surface
- framework-neutral fixed-point and visibility helpers where Rust behavior can
  be matched exactly
- Rust/TypeScript golden fixtures
- Bun package tests and zero-diff regeneration

## Public Behavior

Generated protocol files are artifacts of Rust serde authority. Handwritten
TypeScript may expose ergonomic constructors and pure projections, but cannot
duplicate or reinterpret durable DTOs.

The package performs no import-time browser or Tauri access. It contains no
subscription singleton, Svelte store, Poodle adapter, raw `invoke`, or
`listen`.

## Out Of Scope

- Tauri handler or transport implementation
- subscription handshake
- optimistic mutation store
- Svelte reactivity
- Poodle tabs, dock, or split bindings
- cross-window transfer client
- consumer writes

## Steps

1. Add the minimal binding-generation crate and reproducible selector.
2. Annotate authoritative layout protocol types.
3. Generate checked TypeScript into the owning package.
4. Add package metadata without mirroring Effigy tasks into scripts.
5. Add explicit protocol compatibility metadata.
6. Add framework-neutral ratio and visibility helpers only where exact.
7. Add Rust-produced golden JSON fixtures.
8. Round-trip every command, receipt, error, and snapshot in TypeScript.
9. Add unknown/future variant incompatibility fixtures.
10. Add zero-diff regeneration and Bun validation to Effigy QA.

## Acceptance Criteria

- generated files have one Rust source of truth
- regeneration is deterministic and zero-diff
- every serialized command and result round-trips
- fixed-point ratios remain integers across languages
- unknown future variants fail explicitly
- package import works without `window`, Tauri, Svelte, or Poodle
- no generated file contains donor or product types
- no handwritten duplicate DTO exists
- package-manager-neutral artifact metadata is valid

## Evidence Required

- generation zero-diff check
- Rust/TypeScript golden fixture matrix
- future-version and unknown-variant fixtures
- SSR/import-safety test
- Bun package tests
- Rust 1.85 and full Effigy QA

## Stop Conditions

- generation requires handwritten duplicate contracts
- layout types must import a host or UI package
- a raw Tauri call enters `@longhorn/layout`
- adapter lifecycle from g01.007 enters scope
- package publication names need external registry authority

## Outcome

`longhorn-bindings layout` now emits the checked protocol and golden JSON from
feature-gated Rust authority. Generated compatibility lists come from the
derived enum declarations, so TypeScript does not maintain a second variant
list.

`@longhorn/layout` exports protocol types, explicit compatibility guards,
bounded integer-millionth helpers, and exact ordinary visibility projection.
It is private pending registry verification and imports no host or UI package.

The Rust fixture covers every command, outcome, and rejection discriminant.
Nine Bun tests cover JSON round-trip, integer ratios, Rust/TypeScript
visibility parity, future incompatibility, SSR-safe import, and package
metadata. Zero-diff generation, dry-run packing, focused Clippy, and Rust 1.85
checks pass. Full Effigy QA passes.

## Next Task

Card 027 and `g01.005` are complete. Card 028 is ready under the compiled
`g01.006` runway.
