# 138 Layout Ratio Serde And Schema Caps

Status: complete
Owner: Tom
Roadmap: g02.001 batch 1
Governing refs: contracts 001, 010, and 014; research memo 018
Depends on: none
Auto-start next card: no
Completed: 2026-08-03

## Objective

Enforce the ≤1.0 ratio invariant at every serde entry and cap sizing schema
bounds at 100%.

## Scope

- `crates/longhorn-layout/src/ratio.rs` validating `Deserialize`
- `crates/longhorn-layout/src/definition/validation.rs` schema bound cap
- regression tests for documents, mutations, and definitions
- bindings and fixture regression

## Steps

1. Replace the derived transparent `Deserialize` on `LayoutRatio` with a
   validating impl mirroring `ScaleFactor` in `longhorn-core/src/scale.rs`,
   including its test shape.
2. Add `maximum <= LayoutRatio::ONE` to `validate_schema` with an exact
   validation error variant.
3. Add serde-rejection tests through document, mutation command, and
   definition deserialization; add a schema-cap validation test.
4. Confirm golden fixtures and generated TS bindings are unchanged for valid
   input; regenerate only if byte-identical output is expected.

## Acceptance Criteria

- ratios above 1_000_000 millionths fail deserialization everywhere with a
  typed error
- schemas with `maximum > ONE` fail validation; all existing fixtures pass
- layout suites, `check:layout-bindings`, package checks, and Clippy pass

## Evidence Required

- rejection test receipts across the three serde entries
- unchanged-fixture confirmation
- focused and workspace QA receipts

## Stop Conditions

- a shipped fixture or consumer document legitimately encodes a ratio above
  100% (would mean the invariant is wrong, not the serde path)
- the wire shape must change to satisfy validation

## Evidence

- validating `Deserialize` mirrors `ScaleFactor`; invariant total by
  construction, so the planned `validate_schema` cap is dead code and was
  recorded instead of added
- rejection tests across document, mutation command, and definition serde
  entries
- layout Rust and TS suites, bindings check, package check, Clippy, and
  workspace check pass
- log: `docs/logs/2026-08/03-layout-ratio-serde-and-schema-caps.md`

## Next Task

Promote Card 139.
