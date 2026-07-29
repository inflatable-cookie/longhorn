# Generated Layout TypeScript Protocol

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 026
- added feature-gated `ts-rs` derivation to Rust layout wire types
- added the Rust 1.85-compatible `longhorn-bindings` generator
- generated checked protocol types into `@longhorn/layout`
- derived compatibility discriminants from generated Rust enum declarations
  instead of maintaining a second DTO or variant list
- added explicit protocol-version, command, outcome, and rejection guards
- added bounded integer-millionth ratio helpers
- added exact ordinary region-visibility projection
- added a Rust-produced golden fixture covering every mutation command,
  outcome, and rejection discriminant
- added future-version and unknown-variant incompatibility fixtures
- added SSR/import-safety, metadata, and dry-run package checks
- wired generation, drift, Bun, and package validation through Effigy

## Authority

Rust serde types remain the durable wire authority. Binding derives are
feature-gated, so ordinary `longhorn-core` and `longhorn-layout` consumers do
not depend on `ts-rs`. The generator emits TypeScript declarations,
compatibility metadata, and golden JSON. Checked TypeScript contains no
handwritten durable DTO.

Protocol version 1 is explicit. Known command, outcome, and rejection values
come from `ts-rs` declarations generated from the enums. Adding a Rust variant
therefore changes generated metadata and forces the golden coverage test to
change.

## Package Boundary

`@longhorn/layout` exports generated protocol types, pure compatibility
guards, ratio helpers, and ordinary visibility projection. It has no runtime
dependency, browser global, Tauri call, Svelte store, Poodle binding,
subscription singleton, donor type, or product type.

Transient reveal remains Rust registry policy. The TypeScript visibility
helper covers only occupied and empty ordinary presentation where it can match
Rust exactly.

The package name remains private pending registry verification.

## Evidence

- deterministic write followed by zero-diff check passes
- golden fixture covers all 7 commands, all 7 outcomes, and all 22 rejection
  codes
- all definitions, snapshots, commands, receipts, errors, and visibility
  values survive JSON round-trip
- integer ratio values remain integers and bounded helpers reject invalid
  values
- future protocol version and unknown command, outcome, and rejection values
  fail explicitly
- import succeeds without browser, document, or Tauri globals
- 9 Bun tests pass
- Bun dry-run package contains only README, metadata, and checked source
- focused warnings-denied Clippy passes
- Rust 1.85 workspace all-target check passes
- full Effigy QA passes

## Boundary

No Tauri handler, transport, Surface lifecycle, cross-window transfer, Svelte,
Poodle, product payload, donor write, or consumer migration entered the
package.

## Posture

`strict-ready`

Card 027 is the sole ready lane.

## Next

Review and explicitly start Card 027.
