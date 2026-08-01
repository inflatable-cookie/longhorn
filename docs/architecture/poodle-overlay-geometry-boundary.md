# Poodle Overlay Geometry Boundary

Status: implemented upstream; private artifact admitted for Card 095
Owner: Tom
Updated: 2026-08-01
Governing refs: Poodle contract 002; Longhorn contract 017; Nucleus migration
map and Card 094

## Boundary

Poodle owns the geometry of surfaces created by Poodle components. Longhorn
must not inspect Poodle classes, roles, elements, or portal structure. Nucleus
owns which product overlays affect native Browser visibility. Longhorn owns
the native-content visibility mechanism after it receives explicit policy.

The missing seam is a Poodle callback that reports immutable viewport
snapshots for surfaces owned by one built-in component. It is not a global
overlay registry and has no Tauri, native-content, Browser, or Nucleus
vocabulary.

## Donor Evidence

Nucleus currently wires two intersecting overlays explicitly:

- the project-details `Popover`
- the new-panel `Menu`

Its helper waits one animation frame, then queries
`.poodle-popover__surface, [role="menu"]` below the component's former DOM
ancestor. Poodle contract 002 portals anchored surfaces to the theme root.
The query root therefore cannot contain either live surface after anchoring.
The current implementation can report no intersection even when the visible
surface covers a Browser viewport.

This is a latent donor defect, not behavior to preserve. Migration preserves
the stated policy: only Browser viewports intersected by either explicit
overlay are hidden. It does not preserve the unreachable selector. The
project-manager modal remains a separate explicit hide-all policy.

Poodle has 21 Svelte users of the shared `anchored` action and matching React
composition through `AnchoredSurface`. A callback added only to Nucleus-facing
DOM classes would create another private seam. A global observer would also
change Nucleus behavior by including tooltips, selects, pickers, and other
unregistered overlays.

## Required Poodle Contract

Exact exported names remain Poodle authority. The upstream contract must
provide these semantics:

```ts
interface OverlayViewportRect {
  x: number;
  y: number;
  width: number;
  height: number;
  top: number;
  right: number;
  bottom: number;
  left: number;
}

interface OverlaySurfaceSnapshot {
  surfaceId: string;
  rect: OverlayViewportRect;
  placement: OverlayPlacement | null;
  visible: boolean;
}

type OverlaySurfaceGeometryChange =
  | { type: "upsert"; surface: OverlaySurfaceSnapshot }
  | { type: "remove"; surfaceId: string };
```

- Bounds use viewport CSS pixels and plain numbers. Public values are not
  `HTMLElement`, `DOMRect`, selectors, or mutable browser objects.
- `surfaceId` is opaque and stable only for one mounted surface lifetime.
- `upsert` fires after initial positioning and whenever bounds, resolved
  placement, or visibility changes. Placement-only deduplication must not
  suppress geometry changes.
- `visible` is false for an anchor-hidden or zero-area surface.
- `remove` fires once before the surface disappears or its component is
  destroyed.
- Server rendering emits no surface changes.
- One component callback relays every surface it owns. A `Popover` normally
  owns one. A `Menu` may own a root and nested submenu surfaces.
- Svelte and React expose the same semantic callback on built-in `Popover` and
  `Menu`. The shared anchored primitives carry the underlying observation so
  other components can adopt the same seam without new DOM conventions.

An app-wide provider or registry may be added later. It is not required for
this cutover and must not be the only API: Nucleus needs explicit opt-in scope
to preserve its current product policy.

## Upstream Proof

The Poodle card must update contract 002 plus the Popover and Menu contracts,
then prove in Svelte and React:

- initial geometry arrives after portal positioning
- scroll, window resize, anchor resize, and surface resize update the snapshot
- anchor-hidden and zero-area surfaces report `visible: false`
- close and destruction remove every reported surface exactly once
- nested menu surfaces have independent opaque ids
- public declarations expose no element, selector, or host-runtime type
- existing consumers that omit the callback retain current behavior

The exact Poodle source commit and clean artifact proof are the private
migration gate. Package-manager publication is deferred.

## Nucleus Adoption

Card 099 replaces both selector calls with explicit component callbacks. The
Nucleus adapter keeps a surface map per product overlay and intersects all
visible snapshots with all mounted Browser viewport snapshots. It recomputes
when either side changes; opening time alone is insufficient.

Completed acceptance covers:

- a portalled Popover intersecting one of two Browser viewports
- a Menu moving from non-intersecting to intersecting while open
- one nested menu surface intersecting while its root does not
- close, project switch, and component destruction clearing stale ids
- no private Poodle selector remaining in Nucleus or Longhorn
- the project-manager modal retaining its separate hide-all policy

Nucleus commit `74ca4e7c72f447e064419de6dc72502265cbbf49`
implements the adapter and removes DOM discovery. Popover/Menu and Browser
movement tests pass. Card 099's mounted session proof covers project switch,
late mutation, teardown, and remount.

## Admission Decision

Poodle g12.018 implements this contract at
`ef41f412ad7b45c2ee760c1da9bf41ef876855e8` on
`agent/public-overlay-geometry-observation`. Its clean Svelte artifact proof is
`ed9d800843a5d008a812a29000cbe2fcd3d619ea53e231627a1f253449c4d41d`.

The source/API and private-artifact gates are resolved. Package-manager
publication is not part of Card 095. Nucleus g05 has accepted cross-project
layout retention and the new-project Agent Chat-only default. Card 095 admits
donor writes.
