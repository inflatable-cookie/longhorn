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
- ordered host-window preferences per Surface
- ordered hosted Surfaces and active Surface per participating window

The document contains no display geometry, native window handle, panel body,
product attachment, Poodle state, or evaluated product condition.

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
