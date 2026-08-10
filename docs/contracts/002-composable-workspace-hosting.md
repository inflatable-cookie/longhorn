# 002 Composable Workspace Hosting

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-29
Evidence: `../research/translation-memos/010-surface-hosting-and-transfer-boundary.md`

## Problem

Loophole needs hosted Surfaces. Nucleus found that the same layer duplicated
its project and panel navigation. A shared system must support both without
parallel implementations or mandatory Surface state.

## Hosting Composition

Display inventory and window planning remain independent of layout topology.
Regions belong to an opaque layout container. A consumer chooses one binding:

```text
WindowId -> LayoutContainerId
```

or, with the optional Surface capability:

```text
WindowId -> SurfaceId -> LayoutContainerId
```

Core region and panel APIs do not import Surface types. Nucleus can compose
`longhorn-layout` and `longhorn-windowing` without linking
`longhorn-surfaces`.

## Surface Identity And State

`SurfaceId` uses the bounded opaque-id grammar from `longhorn-core`. It is not
a window label, tab index, layout-container id, or product resource id.

One Surface document carries:

- a monotonic Surface revision
- bounded Surface records
- one distinct `LayoutContainerId` binding per Surface
- an optional mutable display label
- a presentation: regional, or one focused `PanelDefinitionId`
- ordered host-window preferences per Surface
- ordered hosted Surfaces and active Surface per participating window

The document contains no display geometry, native window handle, panel body,
product attachment, Poodle state, or evaluated product condition. A focused
presentation names a panel *definition id* and nothing else — an identifier,
not a body, and not a placement.

### Presentation

A regional Surface renders through its bound container's region tree. A focused
Surface renders one panel full-surface, with no regions and no panel tabs.
Presentation defaults to regional, so a Surface document written before this
clause loads unchanged and no migration is required.

**Longhorn records the focused panel; it does not police the container.** The
Surface domain has no view of container contents — its only evidence about
layout is whether a container exists. Whether a focused Surface's container
holds that panel and only that panel is therefore a consumer obligation, and so
is refusing a panel dropped onto a focused Surface. Both need container and
Surface authority in one place, which no component currently holds.

This is a stated ceiling rather than a silence: a consumer can put a focused
Surface's container into a state the Surface record no longer describes, and
Longhorn will not reject it.

Every Surface resolves to at most one available window. Every active Surface
must be a resolved member of that window. Duplicate Surface ownership,
unknown bindings, incomplete order, and revision overflow fail typed.

## Presence And Resolution

Consumers evaluate product-specific presence predicates and inject the current
admitted Surface set. Longhorn owns deterministic application of that result,
not the predicate language.

Resolution uses:

- the valid Surface document
- current admitted Surface ids
- currently available participating `WindowId`s
- explicit consumer fallback policy

It returns:

- resolved ordered Surfaces per window
- one active Surface per non-empty window
- unresolved Surfaces with typed reasons
- the external `SurfaceId -> LayoutContainerId` binding

Missing preferred windows try declared fallbacks in order. No available host
returns an unresolved Surface. It does not create a window, rewrite the
preferred host, or silently attach to an arbitrary window.

The donor's legacy presence-clause JSON is not shared policy. Product
capabilities, project state, and workflow conditions stay consumer-owned.

## Lifecycle Mutation

Surface mutation is expected-revision, all-or-nothing, and normalized. Commands
cover:

- create a Surface from caller-supplied Surface and layout-container ids
- duplicate generic Surface metadata using caller-supplied fresh ids
- rename a Surface
- set a Surface's presentation to regional or to one focused panel
- activate a hosted Surface
- reorder one window's complete hosted-Surface list
- move a Surface to another declared window host
- close a Surface

Creation and duplication require the caller to prove that the target layout
container exists and is not already bound. Duplication copies only generic
Surface metadata and hosting policy. It does not clone layout contents,
panels, or product resources.

Close removes Surface topology and returns an explicit cleanup intent for the
former layout container. Longhorn does not delete layout or product state by
inference. A consumer policy decides whether a participating window may become
empty; the Loophole donor's last-Surface rule is not a hidden default.

Move and reorder preserve the active member when it remains present. Removing
the active member selects the Surface now at its former index, then the
previous final member. Empty windows have no active Surface.

Move promotes one already-declared candidate window to first host preference
and repositions the Surface in that window's declared order. If the moved
Surface was active in its former primary host, fallback considers the
remaining Surfaces whose first preference is still that host. Close removes
the Surface from every declared candidate order; close fallback considers all
remaining declared members. Target-window active state is preserved when its
active member remains present.

## Persistence

`longhorn-surfaces` is the pure identity, resolution, and mutation package.
`longhorn-surfaces-config` binds one Surface document to an exact registered
configuration domain.

The persistence adapter follows the same rules as `longhorn-layout-config`:

- consumer-supplied descriptor, scope, default, migration, and backup policy
- fresh expected-revision validation under the store coordinator
- immediate complete-document publication for structural mutation
- explicit recovery on future, corrupt, or incompatible state
- no inferred project, user, workspace, or filename

Surface topology, layout state, and window geometry remain distinct domains.
A single command cannot replace one domain with another. Multi-domain cleanup
or cloning is an explicit consumer workflow until a later transaction contract
is promoted.

## Window Host Integration

Surface resolution produces desired bindings for `longhorn-windowing`.
`longhorn-surfaces` does not observe or mutate Tauri windows.

`longhorn-surface-windowing` is the optional pure composition adapter. It
accepts consumer-admitted Surfaces and current placement outcomes, treats only
resolved participating-window placements as available hosts, and projects
plain `DesiredWindow` inputs for the existing host. Direct non-Surface window
outcomes are ignored.

The adapter preserves the selected placement evidence, including temporary
display fallback, without rewriting Surface host preferences or storing
geometry in the Surface document. Native absence and creation capability
remain live host facts: a missing window either returns a typed unsupported
create diagnostic or is created by an injected consumer factory.

The composed host may:

- reconcile resolved participating windows through the existing window host
- wait for hidden placement and page readiness before reveal
- provision a new window only through an explicit consumer policy
- report partial native apply and cleanup receipts
- flush Surface persistence before bounded window-host shutdown

Provisioning a window is not an implicit side effect of panel transfer.

## Layout, Absorbed From Contract 014 — 2026-08-11

Card 179 removed the layout container, so a Surface *is* a layout: it carries
the schema it instantiates, its regions, its sizing slots and the panel
instances placed in them. Contract 014 governed a separate document that no
longer exists, and its substance moves here rather than being restated in two
places. 014 is a superseded stub.

What did not move is 014's Composition section. Its two binding chains were the
container abstraction written down, and they collapse to one:

```text
WindowId -> SurfaceId -> RegionId -> PanelInstanceId
```

## Layout Definition Registry

A consumer registers immutable definitions before resolving or mutating a
document.

### Layout schema

A layout schema declares:

- stable schema id
- complete ordered region definitions
- complete ordered sizing-slot definitions

Region definitions declare:

- region id and family id
- stable order
- empty-region policy: `keep-visible` or `hide-when-empty`
- whether collapse state is supported

Sizing slots declare:

- sizing-slot id
- default, minimum, and maximum ratio

Ratios serialize as integer millionths. The registry rejects inverted bounds,
out-of-range defaults, duplicate ids, duplicate order, empty schemas, and
unbounded counts. A sizing slot is a named consumer seam, not a split-tree
node. The consumer maps it to one or more public Poodle split controls.

### Panel definition

A panel definition declares:

- stable definition id
- ordered default placement selectors
- explicit allowed region ids and/or region families
- explicit instance policy
- movable and closeable policy

Instance policy is an explicit tagged value:

- singleton across the document
- one per Surface
- bounded per document and per Surface
- explicitly multiple within document limits

Missing policy fails registration. Empty allowed placement rejects every
placement. It never means unrestricted.

Definitions contain no title, icon, product attachment, serialized body, or
runtime handle.

## Layout State

A layout document contains:

- revision
- ordered Surfaces
- schema id per Surface
- ordered panel-instance ids per region
- active panel-instance id per region
- collapse state only on supported regions
- current value per registered sizing slot
- panel instance id to definition id

Every panel instance appears in exactly one region of exactly one Surface.
Every referenced schema, region, sizing slot, definition, and instance must
exist. Durable state contains no derived visibility and no product payload.

## Layout Normalization

Current-schema commands operate only on valid input. Corrupt or future state
enters the configuration recovery path instead of being silently repaired.
Explicit migrations may translate older documents before validation.

Successful mutation normalizes:

- Surfaces in stable id order
- regions and sizing slots in registry order
- panel order exactly as committed
- active panel to the requested member, otherwise the first remaining member
- empty regions to no active panel

Closing or moving the active panel selects the panel now occupying its former
index. If the removed panel was last, the previous final panel becomes active.
An empty region has no active panel.

Unknown ids, duplicates, incomplete reorder permutations, invalid sizing
values, and policy violations reject the entire request.

## Region Visibility

Visibility is a projection:

- `keep-visible` regions remain visible when empty
- `hide-when-empty` regions are hidden when empty
- occupied regions are visible
- an optional transient reveal query may expose empty eligible regions during
  a drag

Transient reveal never changes the document or revision. g01.005 does not own
DOM drop zones or cross-window leases.

## Layout Mutation Protocol

Every mutation request carries:

- bounded `LayoutRequestId`
- expected layout revision
- one command

Commands cover:

- create panel instance at a Surface, region, and insertion point
- close panel instance
- activate panel instance
- reorder one region using a complete permutation
- move panel instance across regions or Surfaces
- set one collapsible region state
- set one sizing-slot ratio

Creation uses a consumer-supplied panel instance id. Longhorn never derives
instance identity from time, order, or a panel definition.

Structural commands validate the complete current document and registry,
apply to a private candidate, normalize, revalidate, and then return:

- request id
- previous and committed revision
- authoritative snapshot
- command-specific receipt

A stale revision, rejected policy, invalid command, or failed persistence
returns typed evidence and leaves durable state unchanged. A duplicate request
id is not silently replayed unless the host injects an explicit bounded
idempotency store.

## Layout Persistence

`longhorn-layout-config` is the narrow adapter between `longhorn-layout` and
`longhorn-config`.

The consumer injects:

- exact `DomainDescriptor`
- storage class and relative path
- default document
- definition registry
- backup participation
- any explicit old-schema migration

Longhorn does not infer project ids, user ids, workspace paths, or one
universal layout scope. A consumer may register one aggregate document or
multiple scoped documents.

The adapter:

- loads and validates through the registered configuration domain
- rechecks expected revision against fresh state under store coordination
- publishes one complete document with the existing atomic mutation path
- exposes bounded debounce only for sizing/collapse traffic
- exposes explicit flush
- preserves failed pending intent under the configuration debounce contract

Window geometry uses its own descriptor. Layout mutation cannot replace a
window document or copy renderer-supplied geometry into one.

A changed definition-registry digest requires an explicit compatible
migration. The generic configuration envelope owns schema version. The raw
layout value owns the registry digest and complete document. A current-schema
digest mismatch enters recovery. The consumer must bump the domain schema and
provide the ordered migration hook before Longhorn can reinterpret stored ids
or policy.

## Layout Rust And TypeScript

Rust serde types are authoritative. Contract 010 applies:

- snapshots, commands, receipts, and errors generate checked TypeScript
- generated files live in `@inflatable-cookie/longhorn/layout`
- regeneration must be zero-diff
- unknown future variants fail explicit compatibility checks
- TypeScript cannot invent normalization, placement fallback, or active state

g01.005 supplies protocol types and framework-neutral helpers only. Tauri
transport, subscriptions, Svelte stores, and Poodle adapters remain later
packages.

## Authority

- Rust owns durable Surface resolution and mutation.
- Consumers own product presence predicates, layout-container seeding and
  cleanup, window roles, creation policy, and product resources.
- Renderer projections cannot invent Surface fallback or active state.
- Poodle owns tab interaction and visuals.
- Svelte adapters remain `g01.007`.

## Drag

- same-webview movement may use Poodle HTML5 drag payloads
- cross-window payloads carry only a host-created transfer-session id
- panel transfer targets a current layout container and region
- whole-Surface transfer targets a current participating window
- a no-Surface consumer never imports Surface state
- contract 011 owns session, lease, target, cancellation, and commit rules

## Acceptance

- one fixture resolves `window -> region -> panel`
- one fixture resolves `window -> surface -> region -> panel`
- both fixtures share the layout resolver
- Nucleus-shaped dependencies contain no Surface package
- every Surface has one distinct container binding and at most one resolved
  host
- presence predicates remain consumer-owned inputs
- stale or rejected lifecycle mutation preserves the exact document
- Surface persistence cannot replace layout or window domains
- no native window is created without explicit consumer policy

## Specialized Contracts

- layout definitions, state, mutation, and persistence: contract 014
- configuration envelope and storage coordination: contract 004
- logical and physical coordinates: contract 009
- Rust and TypeScript serialization authority: contract 010
- cross-window target selection and commit: contract 011
- Svelte and Poodle adapter lifecycle: contract 013
