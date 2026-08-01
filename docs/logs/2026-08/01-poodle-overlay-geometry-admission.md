# Poodle Overlay Geometry Admission

Date: 2026-08-01
Roadmap: g01.014
Gate: pre-Card 095

## Result

Defined the Longhorn-side boundary for Poodle-owned built-in overlay geometry,
then completed Poodle g12.018 under its separate upstream card. Nucleus
remained clean and read-only.

The audit found that Nucleus's private selector is structurally incompatible
with Poodle's active anchored-overlay contract. Nucleus queries within the old
component subtree. Poodle portals the live surface to the theme root. The
lookup therefore cannot observe the surface it intends to measure.

The migration target is the intended exact-intersection policy, not the broken
lookup. The upstream request uses explicit component callbacks with immutable
viewport snapshots, opaque per-mount surface ids, visibility, and teardown.
It covers Popover and every Menu surface without exposing elements, selectors,
DOMRect, Tauri, Browser, or Nucleus vocabulary.

## Gate State

- Longhorn boundary: defined
- Poodle public contract and implementation: complete at
  `ef41f412ad7b45c2ee760c1da9bf41ef876855e8`
- clean Poodle Svelte artifact proof:
  `ed9d800843a5d008a812a29000cbe2fcd3d619ea53e231627a1f253449c4d41d`
- exact Poodle private artifact: complete
- Nucleus g05 cross-project layout retention: accepted
- Nucleus g05 new-project Agent Chat-only check: open
- Card 095: planned

Package-manager publication is deferred and is not an admission gate.

## Next

Record the remaining Nucleus g05 new-project check. Then run Card 095 against
the exact private Longhorn/Poodle source and artifact graph. Keep Nucleus
writes planned until both the operator and private artifact gates pass.
