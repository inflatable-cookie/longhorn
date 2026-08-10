# 179 Surfaces Absorb Containers

Status: complete — landed 2026-08-11
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

## Crate Structure — One Crate, And Two Fewer

An earlier revision of this card split the work: one crate would take the state
and the other would keep the definition registry, ratios, limits and state
primitives. That boundary was invented, and it earns nothing.

A Surface *is* a layout. Regions, panels and sizing are what a Surface is, not
a separate domain it references. Checked rather than asserted:

- Two crates depend on `longhorn-layout` without `longhorn-surfaces`:
  `longhorn-transfer` and `longhorn-layout-config`. Both are layout-only
  *because* they use `LayoutDocument`, which is the thing moving. After the
  merge both are surface consumers.
- Nothing uses `LayoutDefinitionRegistry` without a document. Only
  `longhorn-layout`, `longhorn-layout-config` and `longhorn-bindings` reference
  it.
- `longhorn-layout-config` is "registered configuration persistence for
  authoritative layout documents" and `longhorn-surfaces-config` is the same
  sentence for Surface documents. One document, one config crate.

**`longhorn-surfaces` and `longhorn-surfaces-config` are the survivors.**
`longhorn-layout` and `longhorn-layout-config` are absorbed into them and cease
to exist.

The naming follows from what is left in the crate. Once containers are gone,
everything it holds serves Surfaces: the records and their regions, panels and
sizing; the schema registry that describes them; and the hosting policy —
`ParticipatingWindow`, host preferences, the active-Surface selection — which is
not layout under any reading. A crate called `longhorn-layout` would describe
its largest module rather than its subject.

The change removes two crates rather than redistributing code between them,
and a wholesale absorption is *less* risky than the split it replaces: every
module moves, and no judgement is needed about where each one belongs. Module
boundaries inside the surviving crate can still say where layout vocabulary
ends, without a Cargo manifest to enforce it.

`longhorn-surface-transfer` and `longhorn-surface-windowing` keep their names.
`longhorn-transfer` keeps its own, since moving panels between regions is a
different protocol from moving a Surface between windows.

**There is still no compiling intermediate.** The absorption, the document
merge and the `LayoutContainerId` retirement are one atomic change: the moment
`LayoutDocument` stops existing, the dependent crates stop compiling and stay
that way until the sweep completes. There is no half-landed state worth
committing, so this needs one uninterrupted pass rather than staged commits.

## Outcome — 2026-08-11

Landed across three commits: `dfa72456` absorbed the crate, `a4dda1f7` merged
the documents and retired `LayoutContainerId`, and this one adds the stored-state
transform and folds contract 014.

`effigy qa` green: fmt, clippy with `--all-features`, 30 Rust suites, 194
package tests, 104 Svelte tests, `check:bindings`, and all twelve artifact
proofs.

Three invariants stopped being expressible and were removed rather than
weakened. `DuplicateLayoutContainerBinding` rejected two Surfaces sharing a
container, and a schema is shared by design. `UnknownLayoutContainer` and
`LayoutContainerAlreadyBound` became one check that the named schema is
registered. And `CloseSurface` no longer returns a cleanup intent, because the
container it told the caller to clean up was the Surface — the leak Card 178
described cannot happen now.

Two test policies loosened deliberately, both commented where they live. The
no-Surface app-shell shape may import `longhorn/surfaces`, since the document
type it needs lives there. And the layout conformance fixture no longer forbids
`surface_id` in protocol state: layout state is Surface state, so a Surface id
in the document is the subject of the protocol rather than host authority
leaking into it. `window_id` stays forbidden.

`merge_pre_card179_state` in `longhorn-surfaces-config` is the transform. It is
not a `SurfaceMigration` implementation, because the hook is handed one raw
document and this needs both; a consumer calls it from inside its own
`migrate_one`. Every container becomes a Surface, a bound one keeping its
Surface identity, label and hosting policy. Two cases refuse rather than guess:
a Surface naming a container the layout document does not hold, and two
Surfaces sharing one container. Both mean the files were not saved together,
and guessing would silently lose a panel arrangement.

### The blanket substitution bit twice more

The card warned against it for prose. It needed to warn against it for code and
for type names. A regex meant to delete two structs from `model.rs` matched
across the three state primitives between them and left a six-line file. And
`PersistedLayoutDocument` was silently renamed because it contains the
substring `LayoutDocument`. The compiler caught both within a minute, which is
the argument for driving this kind of sweep from `cargo check` rather than from
grep.

## Progress

**Step 1 landed 2026-08-10** (`dfa72456`). `longhorn-layout` is absorbed:
its seven modules live under `crates/longhorn-surfaces/src/layout/`, the crate
is gone from the workspace, and every dependent changed only the crate name in
a `use`. Workspace compiles, lint clean, 25 test suites green,
`check:bindings` and `check:ts` pass.

This contradicts the claim above that there is no compiling intermediate. That
is true of the document merge and false of the crate move — relocating modules
while both documents still exist is mechanical and safe. The claim is left in
place because it is still true of what remains.

**Step 2 attempted and set aside.** The document merge reached a state where
`longhorn-surfaces` itself compiles clean with zero warnings — `SurfaceRecord`
carrying `schema_id`, `regions` and `sizing_slots`, `SurfaceDocument` carrying
`panel_instances`, `LayoutContainer` and `LayoutDocument` deleted, layout
mutation retargeted onto `SurfaceId` and `SurfaceRevision`, `CreateSurface`
materialising regions and sizing slots from the registry, and
`LayoutContainerInventory` replaced by `LayoutDefinitionRegistry` — with **86
errors remaining downstream** across the config, transfer and bindings crates
and their tests.

The work is saved at `~/Dev/docs/patches/179-step2-document-merge.patch`
(1,861 lines) and the tree was restored rather than left broken. Apply it with
`git apply` to resume from that point.

Two things learned in the attempt, both worth having:

- A blanket search-and-replace over the moved layout modules was the wrong
  instrument a second time. A regex intended to delete two structs from
  `model.rs` matched across the three state primitives between them and left a
  six-line file. Restored from git and redone with explicit line boundaries.
  The card's own warning about blanket substitution applies inside the crate,
  not just to prose.
- `DuplicateLayoutContainerBinding` disappears as a validation code. One
  container per Surface was an invariant worth checking; a schema is shared by
  design, so there is nothing left to reject.

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

## Contract 014 Folds Into 002 — Decided 2026-08-10

Contract 014 is titled "Layout Container Region And Panel Core" and its Identity
section leads with `LayoutContainerId`. With containers gone and the crate
absorbed, it governs nothing 002 does not already own: 002 governs the Surface
document, and after this card that document holds the schema, regions, sizing
slots and panel instances 014 describes.

So 014's substance moves into 002 and 014 stops being a live contract.

**It becomes a superseded stub rather than a deletion.** Twelve files reference
it, including two research translation memos and a completed g01 card. Those are
historical records of what governed the work at the time, and rewriting them to
point at 002 would falsify that. A stub also keeps `effigy docs check links`
green and leaves the `qa:northstar:spine` path list in `effigy.toml` unchanged,
so no gate moves.

This is the first retired contract in a set of twenty, so the shape is a
precedent: keep the file, replace the body with a pointer and a one-paragraph
statement of what absorbed it and when, and leave every inbound link working.
It matches how Card 178 was superseded by this card.

**What moves into 002.** The identity set minus `LayoutContainerId`, the
definition registry, durable state, normalization, visibility, the mutation
protocol and the persistence clauses. What does not move: 014's Composition
section, whose two binding chains are the container abstraction stated
outright, and which collapses to one chain — `WindowId -> SurfaceId -> RegionId
-> PanelInstanceId`.

## Steps

1. Move `schema_id`, `regions` and `sizing_slots` onto `SurfaceRecord`; move
   `panel_instances` onto `SurfaceDocument`. Delete `LayoutContainer`.
2. Retarget the layout mutation commands and their rejections onto `SurfaceId`.
3. Absorb `longhorn-layout` into `longhorn-surfaces` and
   `longhorn-layout-config` into `longhorn-surfaces-config`; drop
   `LayoutContainerInventory` and the container rejection codes.
4. Move the Card 177 container invariant and the panel-drop guard into the
   engine as local checks, and delete contract 002's stated ceiling.
5. Retire `LayoutContainerId` from `longhorn-core`.
6. Regenerate bindings; sweep the TypeScript protocol, clients and validation.
7. Rewrite the fixtures, including the no-Surface conformance shape as a
   single unlabelled Surface.
8. Provide the stored-state transform and test it both ways.
9. Sweep examples and proofs. Fold contract 014 into 002 and leave 014 as a
   superseded stub.

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
- `longhorn-layout` and `longhorn-layout-config` no longer exist
- a Surface carries its own schema, regions and sizing slots
- the no-Surface conformance shape is one unlabelled Surface and still passes
- a focused Surface's single-panel invariant is enforced by the engine
- a stored layout-plus-Surface pair transforms into one Surface document
- contract 014 is a superseded stub and every inbound link still resolves
- `effigy qa` green, including `check:bindings` and all twelve artifact proofs

## Why Before The Tag

Longhorn is unpublished and all six of its consumers pin it by `file:`, so
nothing external pays for the delay. The publication work exists to stop
shipping names that would need deprecating — Card 164 took eighteen packages to
three, and `poodle-react` is held back for the same reason. `LayoutContainerId`
on a public wire is that mistake, caught before first publication instead of
after.
