# Live Window Diff Planning

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed `g01.003`
- added pure desired and live window snapshots
- separated stable `WindowId` from opaque host transport handles
- added explicit create, retag, unmaximize, move/resize, maximize, show, hide,
  focus, and close operations
- added deterministic phase and stable-id ordering
- added explicit protected-primary preserve and reuse policy
- added typed host capabilities and unavailable-operation diagnostics
- attached caller apply generations to every operation
- added feedback evidence for rejecting stale programmatic events

## Planning Policy

Stable `WindowId` matches desired and live state. Host labels remain opaque
transport handles and are never parsed into domain identity. Duplicate stable
ids or handles reject the snapshot before planning.

Protected primary slots are named explicitly. Preserve leaves the slot alone.
Reuse emits an explicit retag before geometry. Missing slots, missing targets,
and conflicting stable matches return diagnostics. Protected slots never
enter inferred close planning.

Creation and retag run first. Unmaximize precedes normal geometry;
move/resize compares desired outer origin and inner content size against the
matching live facts. Live outer extent is ignored for placement equality.
Maximize follows normal geometry. Visibility and focus follow geometry. Stale
windows close last in stable-id order.

Unsupported operations are omitted and reported. A failed create suppresses
dependent mutations for that absent slot. A failed unmaximize suppresses the
normal-geometry operation that depends on leaving maximized state.

## Donor Evidence

- Loophole protected bootstrap reuse becomes explicit retag policy
- Loophole maximize restoration preserves normal geometry before maximizing
- Nucleus no-Surface windows and Loophole hosted-Surface windows produce the
  same native plan
- transport labels shaped like another window id cannot redirect matching
- donor host behavior remains evidence; no donor type enters the public API

## Evidence

- 16 focused desired/live planner fixtures pass
- input permutation produces identical receipts
- already matching live state produces an empty receipt
- outer-frame extent cannot stand in for inner content size
- create, retag, geometry, maximize, visibility, focus, and close ordering is
  covered
- protected reuse, conflict, unidentified live slots, duplicate identity,
  capability failure, generation propagation, and stale feedback are covered
- input and receipt serde round trips pass
- normal dependencies remain `longhorn-core`, `longhorn-display`, and `serde`
- Rust 1.85 workspace check, formatting, warnings-denied Clippy, workspace
  tests, and Effigy QA pass
- Effigy Doctor reports zero errors and no new size warning

## Boundary

No Tauri call, native mutation, event listener, debounce, settling,
persistence, configuration update, layout, Surface state, TypeScript, Svelte,
Poodle, product type, or donor write entered the package.

## Posture

`strict-ready`

Cards 013-016 and `g01.003` are complete. The later `g01.004` compilation made
Card 017 ready.

## Next

Review and explicitly start Card 017.
