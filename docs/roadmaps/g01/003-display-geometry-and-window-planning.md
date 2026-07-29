# g01.003 Display, Geometry, And Window Planning

Status: complete
Owner: Tom  
Updated: 2026-07-28
Governing refs: contract 009

## Outcome

Extract pure display correlation, coordinate, geometry, and desired-window
planning without Tauri or Surface dependencies.

## Goals

- [x] Coordinate types prevent physical, screen-logical, and client-local
  substitution.
- [x] Known display identity survives absence and rearrangement without
  promoting weak host evidence.
- [x] Window placement and live diffing remain pure, deterministic, and
  independent of Surfaces.
- [x] Donor cases become consumer-neutral fixtures rather than shared product
  schemas.

## Execution Plan

- [x] Batch 1 — [Card 013](batch-cards/013-typed-coordinate-and-geometry-foundation.md):
  typed ids, coordinate spaces, scale, checked geometry, clamping, and property
  tests in `longhorn-core`.
- [x] Batch 2 — [Card 014](batch-cards/014-display-inventory-and-correlation.md):
  persistent known displays, current observations, evidence-bearing
  correlation, ambiguity, availability, labels, and arrangement signatures in
  `longhorn-display`.
- [x] Batch 3a — [Card 015](batch-cards/015-window-placement-resolution.md):
  configured targets, ordered fallbacks, per-display normal geometry,
  maximized separation, clamping, and settled-placement proposals in
  `longhorn-windowing`.
- [x] Batch 3b — [Card 016](batch-cards/016-live-window-diff-planning.md):
  desired/live snapshots, protected primary reuse, explicit ordered operations,
  apply generations, and host diagnostics.

## Acceptance Criteria

- [x] Loophole correlation, configured fallback, and frame-distinction cases
  pass.
- [x] Nucleus saved, intersection, main, deterministic, and unavailable
  fallback cases pass.
- [x] Soundcheck single-window minimum-size and work-area clamp cases pass.
- [x] Input permutations cannot change correlation, signatures, fallback
  choice, or operation order.
- [x] Temporary fallback never silently changes a configured home display.
- [x] No public geometry API accepts an untyped tuple or rectangle.
- [x] Pure package graphs contain no Tauri, Svelte, Poodle, Surface, or product
  dependency.

## Lane Runway

- Generation goal: establish the pure display/window foundation required by
  `g01.004`, `g01.005`, and both simple and full-host consumer migrations.
- Ready now: Card 018 in `g01.004`.
- Completed foundation: Cards 013-016; `g01.003` closed.
- Planning checkpoint: `g01.004` is compiled through Card 022.

## Planning Gaps

- Strong non-macOS display evidence remains a host-adapter research item for
  `g01.004`. It does not block optional evidence in the pure correlation seam.
- Ambiguity presentation remains consumer/settings work. Card 014 returns
  evidence and candidates only.
- Packaged Tauri mutation, event attribution, settling, and close flush remain
  `g01.004`.

## Current Gate

`g01.003` and Card 017 are complete. Card 018 is the sole ready `g01.004`
lane.
