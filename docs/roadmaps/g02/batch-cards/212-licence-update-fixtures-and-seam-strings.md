# 212 Licence/update Fixtures And Seam Strings

Status: ready
Owner: Tom
Roadmap: g02.024 batch 2
Governing refs: contract 010; contract 019; memo 023 (TS-M1, TS-M2, M4
fixtures)
Depends on: none
Auto-start next card: no

## Objective

The two newest protocols get the neutral fixtures the thirteen older ones
have, the boundary test cannot silently miss a new domain again, and the last
hand-written seam — `longhorn-tauri`'s invoke/event strings — gets a
mechanical check.

## Why this exists

- `crates/longhorn-bindings/src/licence.rs` and `update.rs` have no
  `GOLDEN_FIXTURE` const; the thirteen older domains all do
  (`fixtures/<domain>/protocol-v1.json`). The newest protocols lack the
  cross-language fixture — ironic against `licence.rs:24`'s own comment that
  "the only defence is a fixture neither side authors."
- `packages/longhorn/tests/boundary.test.ts:12-28` enumerates 14 domains and
  omits `licence` and `update`; the export loop iterates `exports` itself, so
  a missing root never fails. The drift the test exists to catch, happening
  inside the test.
- `packages/longhorn-tauri`'s raw ports hand-write command/event strings
  (e.g. `licence.ts:11-17` mirrors `crates/longhorn-tauri-licence/src/
  commands.rs:13`); the tests use the exported constants on both sides of the
  fake transport, so a typo passes everything. No proof app exercises the
  `longhorn-tauri-*` handler seam end-to-end.
- Dead `svelte-shims.d.ts` files in the peerless package weaken the boundary
  guarantee in a way the test's needles miss.

## Scope

- `crates/longhorn-bindings` — two new golden fixtures
- `packages/longhorn/tests/boundary.test.ts` — derived domain list
- a conformance check for the tauri seam strings
- deletion of the two dead shim files

## Steps

1. Add `fixtures/licence/protocol-v1.json` and
   `fixtures/update/protocol-v1.json`, gated by `check:bindings` like the
   other thirteen.
2. Boundary test: derive the domain list from the `src/` directory (or assert
   exports ⊇ src dirs) so adding a domain without touching the test fails.
3. Seam strings: extend `longhorn-bindings` to emit the command/event name
   constants from Rust, or add a conformance script comparing
   `crates/longhorn-tauri-*/src/commands.rs` constants against the TS
   exports. Pick by which one the next new domain would rather inherit.
4. Delete `packages/longhorn/src/history/svelte-shims.d.ts` and
   `src/history-tree/svelte-shims.d.ts`; add
   `/// <reference types="svelte"` to the boundary test's forbidden needles.

## Do Not

- Hand the seam check to the proof apps "later". The string check is the
  cheap 80%; end-to-end handler coverage is Card 214's, separately.

## Acceptance Criteria

- [ ] both protocols have gated golden fixtures
- [ ] the boundary test needs no edit when a domain is added — and fails when
  a domain's root is missing
- [ ] a typo in a tauri invoke string fails a gate
- [ ] the dead shims are gone and the needle catches their return

## Evidence Required

- the fixtures and their gate
- the failing-then-passing demonstration of each new check
- `effigy qa` green

## Stop Conditions

None anticipated.
