# 027 Two-shape Layout Conformance And Closeout

Status: complete
Owner: Tom
Roadmap: g01.005 batch 3
Governing refs: contracts 001-004, 010, 012, and 014; research memos 001 and 009
Auto-start next card: no

## Objective

Prove one layout model, mutation engine, persistence adapter, and TypeScript
protocol against Loophole and Nucleus shapes, then close g01.005 without
claiming donor migration.

## Scope

- Loophole eight-region fixture with an external Surface-to-container binding
- Nucleus five-region fixture with an external window-to-container binding
- shared create, close, activate, reorder, move, sizing, collapse, and
  visibility sequences
- singleton and multi-instance differences
- independent window/layout persistence concurrency
- Rust/TypeScript fixture parity
- package-boundary and dependency audit
- roadmap, contract, architecture, and evidence closeout

## Public Behavior

Both fixtures use the same public Longhorn engine. The Loophole adapter may
carry `SurfaceId`, and the Nucleus adapter may carry `WindowId`, but neither
identity enters the shared layout document or package.

Fixtures preserve product differences:

- Loophole: eight regions, singleton-heavy definitions, three sizing controls
- Nucleus: five regions, activity/workspace families, four sizing controls,
  singleton Tasks, multiple tool instances

Conformance proves foundation behavior only. Contract 003 ownership transfer
remains incomplete until consumer migration lanes remove donor copies.

## Out Of Scope

- modifying Loophole or Nucleus
- optional Surface implementation
- cross-window drag
- Svelte or Poodle adapters
- packaged desktop UI
- donor API compatibility shims

## Steps

1. Freeze schema-neutral fixture inputs and expected snapshots.
2. Adapt Loophole Surface identity outside the core document.
3. Adapt Nucleus window identity outside the core document.
4. Run one shared structural mutation sequence.
5. Run distinct instance-policy cases.
6. Run sizing, collapse, empty visibility, and transient reveal cases.
7. Run stale, invalid, and concurrent persistence cases.
8. Verify Rust/TypeScript golden parity.
9. Audit the complete package graph for forbidden dependencies.
10. Record behavior retained, intentionally changed, and deferred.
11. Close Cards 023-027 and g01.005.
12. Stop at the g01.006 planning checkpoint.

## Acceptance Criteria

- both fixtures use the same public resolver and mutation engine
- no Surface type appears in core or Nucleus fixture state
- no Window type appears in the core layout document
- donor-specific region ids remain fixture configuration
- product resource attachments remain outside panel instance state
- complete behavior matrix passes for both shapes
- layout/window concurrent writes preserve both domains
- Rust and TypeScript expected snapshots are equal
- Rust 1.85 and full Effigy QA pass
- docs name unexecuted migration and UI limits

## Evidence Required

- checked donor-shaped fixture files
- shared conformance matrix
- package dependency report
- cross-language fixture report
- independent-domain concurrency proof
- behavior delta table
- milestone closeout log
- Rust 1.85 and full Effigy QA

## Stop Conditions

- conformance requires importing donor crates or source
- fixture adapters become hidden compatibility layers
- a consumer repository must change
- Surface, transfer, Svelte, Poodle, or Tauri behavior is required
- g01.006 contract or package boundaries become unclear

## Outcome

Rust-generated Loophole and Nucleus fixture files now preserve their eight/three
and five/four region/sizing shapes. Both run the same public default resolver
and eight-step mutation matrix. External host bindings carry fixture-only
Surface or window identity; neither enters definitions, commands, receipts, or
snapshots.

The matrix covers multiple and singleton policy, create, close, activate,
reorder, move, sizing, collapse, ordinary visibility, transient reveal,
stale rejection, invalid rejection, and exact unchanged evidence. TypeScript
consumes the Rust expected snapshots without duplicating mutation behavior.

Existing configuration conformance proves concurrent layout and window
domains preserve both values. Dependency inspection keeps the layout core on
`longhorn-core` and serde only. No donor repository changed.

## Next Task

Contract 011 and the `g01.006` runway are compiled. Card 028 is ready; start it
explicitly from its own card.
