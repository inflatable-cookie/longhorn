# g02.037 Focused Panel Surfaces Closeout

## Outcome

A Surface can present one panel full-surface. `SurfaceRecord` carries a
`presentation` — `regional`, or `focused_panel` with a `PanelDefinitionId` — set
by `SetSurfacePresentation` with a typed `UnknownSurface` rejection. The field
defaults to `regional`, so a document written before the change loads unchanged.

## Evidence

- Implementation and tests: `e4903980` (2026-08-10, Card 177). `effigy qa` was
  green at that commit, including `check:bindings` and all twelve artifact
  proofs.
- Contract 002 states the container invariant — a focused Surface's container
  holds exactly that panel — as a consumer obligation, and states plainly that a
  consumer can put a container into a state the Surface record no longer
  describes.
- Bindings and fixtures are current on `main`. The presentation mutation tests
  live in
  `crates/longhorn-surfaces/tests/surface_contract/mutation/presentation.rs`.

## Material limits

- Longhorn records the focused panel; it does not police container contents.
  Widening `LayoutContainerInventory` to carry panel membership was considered
  and rejected: it would make every caller assemble panel membership for every
  container in order to change one label.
- Refusing a panel dropped onto a focused Surface belongs to panel transfer, a
  composition-layer concern, and stays a consumer obligation.

## Queue disposition

The lane was dispatched to northstar-queue on 2026-09-15 against work that had
already merged. The worker found a zero diff and no PR possible. Queue task
`641c2d5a-ff81-4a43-8ce7-3d8285eb5296` was cancelled with that disposition, and
the stale dispatch handoff was removed. This log is the canonical closeout
because no merge exists to close.
