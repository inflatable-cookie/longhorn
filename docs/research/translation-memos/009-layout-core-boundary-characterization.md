# Layout Core Boundary Characterization

Status: promoted
Owner: Tom
Updated: 2026-07-28

## Question

Which layout behaviors are shared by Loophole and Nucleus, and which topology,
presentation, and product concerns must stay outside `longhorn-layout`?

## Repositories

- `loophole/echo` and `loophole/aura`
- `nucleus/crates/nucleus-workspaces`
- `nucleus/apps/desktop`
- `poodle`

Inspection was read-only. Donor worktrees were not modified.

## Loophole Evidence

Loophole supplies the advanced eight-region shape:

- Surface-bound layout containers
- top, bottom, left, and right strips
- left and right docks
- primary and secondary center regions
- panel definitions with allowed regions and singleton/multi-instance policy
- stable panel ordering and active-panel selection
- size and collapse state on the three resizable regions
- same-webview Poodle drag with Rust-hosted placement mutation

The current donor remains product-shaped:

- `SurfaceId` is embedded in region and placement records
- the region enum is fixed to Aura's eight regions
- most panel keys are singleton DAW tools
- fallback placements mix layout and Surface hosting policy
- repair code accepts historical schema drift

Those shapes are evidence, not the shared contract.

## Nucleus Evidence

Nucleus supplies the no-Surface five-region shape:

- a window is the direct layout host
- `left` is an activity family
- four center/right regions form the movable workspace family
- panels have stable instance ids separate from panel kinds
- Tasks is singleton; other tool kinds may have multiple instances
- active tabs, close, reorder, cross-region move, and empty-region reveal exist
- four persisted ratios drive a fixed Poodle split composition
- layout state is project-keyed local client state

The current desktop mutates full renderer snapshots and writes them
asynchronously. Requests have no expected revision. Product resource targets
are stored inside panel records. These are migration problems, not Longhorn
contracts.

## Shared Boundary

Both donors support one smaller model:

- an opaque layout-container id
- a consumer-selected layout schema
- flat semantic regions with families and stable order
- panel definitions separate from panel instances
- explicit default and allowed placement
- explicit instance-count, close, and move policy
- one ordered instance list and active instance per region
- bounded sizing values and collapsible regions
- deterministic create, close, activate, reorder, and move

Neither donor proves a reusable arbitrary split tree. Poodle owns split and
dock presentation. Consumers own the mapping from shared region state into
their Poodle composition.

## Promoted Decisions

- `longhorn-layout` has no `SurfaceId`, `WindowId`, Poodle, Svelte, Tauri, or
  product dependency.
- Host adapters bind a window or Surface to `LayoutContainerId`.
- Region schemas are consumer-registered and flat.
- Named sizing slots store fixed-point ratios with declared bounds. They do
  not encode a generic split tree.
- Empty-region visibility is derived from region policy, occupancy, and an
  optional transient reveal query. It is not independent durable truth.
- Missing placement or instance policy fails registration. An empty
  allowed-region set never means unrestricted placement.
- Panel instance records contain layout identity only. Product resources,
  titles, bodies, and runtime handles stay consumer-owned.
- Mutations carry an expected revision and commit one complete normalized
  snapshot or no state.
- Structural mutation is host-authoritative. Renderer projection may be
  optimistic but must reconcile to the returned revision.
- Consumers inject the exact configuration descriptor and scope. Layout state
  cannot share a whole-file write path with window geometry by accident.
- Rust owns serialized snapshots and commands. TypeScript is generated and
  drift-checked before Svelte/Poodle adapters arrive.

## Deferred Work

- Surface lifecycle and hosting preferences: `g01.006`
- cross-window transfer sessions and target leases: contract 011 and
  `g01.006`
- Svelte stores, Poodle bindings, and transient drag presentation: contract
  013 and `g01.007`
- donor migration and ownership transfer: `g01.014` and `g01.015`
- arbitrary split trees: no current contract

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/package-topology.md`
- `../../contracts/002-composable-workspace-hosting.md`
- `../../contracts/014-layout-container-region-and-panel-core.md`
- `../../roadmaps/g01/005-layout-container-region-and-panel-core.md`
- Cards 023 through 027
