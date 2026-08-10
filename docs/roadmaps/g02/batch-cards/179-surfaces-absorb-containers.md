# 179 Surfaces Absorb Containers

Status: ready
Owner: Tom
Roadmap: g02 planning checkpoint
Governing refs: contract 002; contract 014
Depends on: none
Blocks: Longhorn's first tag
Supersedes: Card 178
Auto-start next card: no

## Objective

`LayoutContainerId` leaves the model. A Surface owns its own layout — schema,
regions, sizing slots — and a consumer that wants no surface concept uses one
Surface with no label and no visual representation.

## Why This Exists

The container is not part of the designed vocabulary. It is the seam left by
building the layout domain before the Surface domain existed, and every
cross-domain problem this generation has hit traces back to it.

The evidence says it earns nothing:

- `ParticipatingWindow.active_surface_id` is a single `Option<SurfaceId>`. One
  active Surface per window, so two containers can never be visible at once.
- Every layout fixture holds exactly **one** container — including
  `window-bound-conformance-v1.json`, the no-Surface conformance shape, where
  it is named `container:primary`. The whole justification for the abstraction
  is the `WindowId -> LayoutContainerId` chain, and it is exercised by exactly
  one container per document. That is already "one implicit Surface".
- The only multi-container document is the Surface fixture, where containers
  map one-to-one onto tabs.
- The binding is created once and never rebound. No command changes
  `layout_container_id`; it is 1:1 and exclusive.

Two names, one thing, and one of the names was never designed.

## What Dissolves

- **Card 178** entirely. Creating a Surface creates its layout from a schema;
  closing one removes it. There is no separate container to provision.
- `LayoutContainerCleanupIntent`, and the container leak it described. Closing
  a Surface no longer hands the caller work the protocol cannot perform.
- Card 177's container invariant. Presentation and panels land in one record,
  so "a focused Surface holds exactly that panel" becomes a local check.
- The panel-drop guard, for the same reason. Both stop being consumer
  obligations, and contract 002's stated ceiling can be deleted rather than
  documented around.
- `LayoutContainerAlreadyBound`, `UnknownLayoutContainer`,
  `LayoutContainerInventory`, and the evidence parameter every caller of the
  Surface mutation engine currently assembles.
- The composition-layer owner that three cards asked for. Nothing left to
  compose.

## Target Model

```text
SurfaceDocument { revision, surfaces, panel_instances, windows }
SurfaceRecord   { id, schema_id, label, presentation,
                  regions, sizing_slots, host_preferences }
```

Panel instances stay a flat document-level list referenced by region, as today.
Layout commands target `SurfaceId + RegionId` where they took
`LayoutContainerId + RegionId`.

A Surface with no surface-ness is `label: None`, `presentation: regional`, one
host preference. The host preference stays required: a layout renders
*somewhere*, and naming the window is one line of honesty rather than an
implicit default.

## The Real Cost: Two Revisions Become One

This is the argument against, and it should be recorded rather than discovered
later.

`LayoutRevision` and `SurfaceRevision` are independent today. A consumer can
move a panel and rename a Surface concurrently and neither expected-revision
check sees the other. After the merge there is one revision, so any layout
mutation conflicts with any Surface mutation on the same document.

For a desktop application driving both from one process this is almost
certainly irrelevant — mutations are already serialized through one authority
and rejections carry the authoritative document for immediate retry. It would
matter if two independent agents mutated the two domains concurrently, which
nothing in the portfolio does.

Accepted deliberately. If contention ever appears, the answer is a finer-
grained expected-revision check, not a second document.

## Crate Structure — The Move Is Forced

Attempted 2026-08-10 and reverted before committing, which established the
shape of the work.

`longhorn-surfaces` keeps the document and absorbs everything stateful:
`layout_mutation` (engine, protocol, operations, error, replay — 834 lines),
document validation (390), the document and record model, and visibility.
`longhorn-layout` is left holding the definition registry, ratios, limits and
the region/panel/sizing state primitives, and `longhorn-surfaces` gains a
dependency on it.

This is not a preference. One crate must own the merged document, and both the
Surface engine and the layout engine mutate it. If the document lived in
`longhorn-layout`, `longhorn-surfaces` would depend on it for the document
while `longhorn-layout` depended on `longhorn-surfaces` for the record type.
Neither crate depends on the other today, and only one direction can survive.

**There is no compiling intermediate.** The move, the document merge and the
`LayoutContainerId` retirement are one atomic change: the moment
`LayoutDocument` stops existing, twelve crates stop compiling and stay that way
until the sweep completes. There is no half-landed state worth committing, so
this needs one uninterrupted pass rather than staged commits.

The alternative is one merged crate. Cheaper to execute, and it loses the
ability to state where layout vocabulary ends.

## Migration

Unlike Card 177 this is not a defaulted field, so it needs a real step.
Consumers own the schema version through their `SurfaceMigration` hook, and the
stored shape changes on both sides: the layout document disappears and the
Surface document grows.

Longhorn should provide the transform rather than leaving each consumer to
write it — join the two stored documents on `layout_container_id`, move regions
and sizing slots onto the Surface, and carry panel instances across. A consumer
with a layout document and no Surface document gets one Surface per container,
unlabelled.

## Steps

1. Move `schema_id`, `regions` and `sizing_slots` onto `SurfaceRecord`; move
   `panel_instances` onto `SurfaceDocument`. Delete `LayoutContainer`.
2. Retarget the layout mutation commands and their rejections onto `SurfaceId`.
3. Fold the layout mutation engine into `longhorn-surfaces`; drop
   `LayoutContainerInventory` and the container rejection codes.
4. Move the Card 177 container invariant and the panel-drop guard into the
   engine as local checks, and delete contract 002's stated ceiling.
5. Retire `LayoutContainerId` from `longhorn-core`.
6. Regenerate bindings; sweep the TypeScript protocol, clients and validation.
7. Rewrite the fixtures, including the no-Surface conformance shape as a
   single unlabelled Surface.
8. Provide the stored-state transform and test it both ways.
9. Sweep examples and proofs; update contracts 002 and 014.

## Sweep Order

The rename is most of the work and the compiler drives it. Do the Rust model
first and let `cargo check --workspace` enumerate the rest — 12 crates touch
`LayoutContainerId`. Only then regenerate bindings, because the TypeScript
surface is derived and re-deriving it early just means doing it twice.

Do not run a blanket text substitution across prose and code. Card 167 did
exactly that and broke a release gate and two crates; `container` appears in
documentation where it means the ordinary English word.

## Measured 2026-08-10

`LayoutContainerId`, `layout_container_id` or a `container:` literal appears in
118 files: 69 in `crates`, 24 in `packages`, 10 in `examples`, 7 in `fixtures`,
8 in `docs`. Twelve crates reference the type.

Most of it is mechanical. The restructure is confined to the two model modules
and the two mutation engines.

## Acceptance Criteria

- no `LayoutContainerId` in any crate, package, fixture or example
- a Surface carries its own schema, regions and sizing slots
- the no-Surface conformance shape is one unlabelled Surface and still passes
- a focused Surface's single-panel invariant is enforced by the engine
- a stored layout-plus-Surface pair transforms into one Surface document
- `effigy qa` green, including `check:bindings` and all twelve artifact proofs

## Why Before The Tag

Longhorn is unpublished and all six of its consumers pin it by `file:`, so
nothing external pays for the delay. The publication work exists to stop
shipping names that would need deprecating — Card 164 took eighteen packages to
three, and `poodle-react` is held back for the same reason. `LayoutContainerId`
on a public wire is that mistake, caught before first publication instead of
after.
