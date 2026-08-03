# g02.001 Layout Sizing Integrity

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 010, and 014; research memo 018
Depends on: none

## Outcome

Make every serde path into layout state enforce the same sizing invariants
the constructors enforce, so no document, mutation command, or definition can
materialize a ratio above 100% or a schema whose bounds exceed it.

## Generation Runway

First g02 milestone. Bounded to `longhorn-layout` plus regenerated bindings
and fixtures; no protocol field changes.

## Execution Plan

### Batch 1. Validating deserialization and schema caps

- [x] [Card 138](batch-cards/138-layout-ratio-serde-and-schema-caps.md)
  gives `LayoutRatio` a validating `Deserialize` and caps sizing schema
  bounds at 100%

## Goals

- [x] reject out-of-range ratios at every serde entry, not only constructors
- [x] reject sizing schemas whose maximum exceeds `LayoutRatio::ONE`
- [x] keep wire shape, TS bindings, and golden fixtures byte-compatible for
  valid input

## Acceptance Criteria

- [x] deserializing a ratio above 1_000_000 millionths fails with a typed
  serde error in documents, mutations, and definitions
- [x] `maximum > ONE` is unrepresentable by construction (validating serde
  closed the only bypass); existing valid fixtures pass unchanged
- [x] layout, bindings, and package QA pass

## Explicit Non-goals

- layout protocol version bump
- consumer repository edits

## Next Task

Promote Card 139 (g02.002).
