# 215 Post-179 Architecture Sweep

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.025 batch 1
Governing refs: contract 002; contract 014; memo 023 (C2)
Depends on: none
Auto-start next card: no

## Objective

The canonical architecture documents describe the Surface-as-layout model
that exists, and contract 002 reads as one contract instead of two drafts
stapled together.

## Why this exists

Card 179 (2026-08-10/11) removed `LayoutContainerId` and absorbed
`longhorn-layout`/`-config` into `longhorn-surfaces`/`-config`. The code
moved; the canonical docs did not:

- `docs/architecture/system-architecture.md:19-37,74-93,160` — hosting model
  and the entire "Layout core" layer built on the opaque layout container;
  both deleted crates named as implementers.
- `docs/contracts/002-*.md` — the absorbed 2026-08-11 section was appended;
  the body still chains `WindowId -> LayoutContainerId` (`:17-31`), claims
  Nucleus composes `longhorn-layout` without `longhorn-surfaces` (`:17-31`,
  falsified by `examples/nucleus-no-surface-proof/Cargo.toml:11`), names
  `longhorn-layout-config` as the adapter (`:339`), and points layout
  substance at contract 014 (`:421`) whose stub points back at 002 — a
  pointer loop. Header still says `Updated: 2026-07-29`.
- `docs/architecture/package-topology.md:31-32,354-374` lists the deleted
  crates and omits existing ones (`longhorn-credential-keyring`,
  `longhorn-poodle`, `longhorn-tauri-licence`, the recreated
  `longhorn-tauri-update`); narrates `longhorn-tauri-update`'s absorption as
  the end state (`:72-77`); lists a `tools/` directory that does not exist
  (`:13`).
- `docs/architecture/system-inventory.md:150` still says "layout containers";
  `crates/longhorn-surfaces/tests/surface_contract/mutation/presentation.rs:16`
  still says the engine "borrows its container inventory".
- `docs/guides/system-composition.md:55,58,64` teaches the removed hierarchy.

## Scope

- the four architecture docs and contract 002/014 named above
- the one stale test comment
- `nucleus-no-surface-proof`: rename the example or restate the criterion —
  the name now asserts something false

## Steps

1. Rewrite `system-architecture.md`'s hosting and layout sections against
   the `WindowId -> SurfaceId -> RegionId -> PanelInstanceId` chain contract
   014's stub states.
2. Revise contract 002's pre-absorption body to agree with its absorbed
   section; break the 002↔014 pointer loop by stating where layout substance
   lives, once.
3. Resolve the `:411` acceptance criterion: the proof now depends on
   `longhorn-surfaces` (one unlabelled Surface), so either rename the proof
   to stop asserting "no Surface" or restate the criterion to what it
   actually proves. Record the choice.
4. Correct `package-topology.md` (crate table, dependency diagram, tools/
   line, the tauri-update narrative) and `system-inventory.md:150`.
5. Re-teach `system-composition.md`'s hierarchy.
6. Fix the test comment.
7. If Card 211 has landed, fold in its recorded divergences as contract
   amendments here.

## Do Not

- Touch handoffs, logs, or research memos that reference `longhorn-layout` —
  historical records stay historical.
- Rewrite the acceptance criterion to match the proof silently; the criterion
  said something real once, and the record should show why it changed.

## Acceptance Criteria

- [x] `LayoutContainerId` appears in live canonical docs only as history
- [x] contract 002's body and absorbed sections cannot be quoted against each
  other
- [x] the 002↔014 loop is broken
- [x] `package-topology.md`'s crate table equals the workspace members

## Evidence Required

- the diffs; a grep showing zero live references to the deleted model outside
  history
- the criterion/proof resolution recorded

## Stop Conditions

Stop if the sweep finds the absorbed model itself under-specified (Card 211's
divergence list is the early warning) — that pauses here for a contract
conversation, per the milestone checkpoint.
