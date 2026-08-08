# 014 Layout Container, Region, And Panel Core

Status: implemented foundation; consumer adoption pending
Owner: Tom
Updated: 2026-07-28
Depends on: contracts 001, 002, 004, 010, and 012
Evidence: `../research/translation-memos/009-layout-core-boundary-characterization.md`

## Boundary

Longhorn owns product-neutral layout identity, registered placement policy,
normalized durable state, and authoritative mutation.

Consumers own:

- the binding from a window or Surface to a layout container
- region names, families, schema choice, and Poodle composition
- panel catalogue labels, bodies, product resources, and runtime cleanup
- the persistence scope and exact configuration descriptor

`longhorn-layout` imports no window, Surface, configuration, host, TypeScript,
Svelte, Poodle, Tauri, or consumer type.

## Identity

The shared identity set is:

- `LayoutSchemaId`
- `LayoutContainerId`
- `RegionId`
- `RegionFamilyId`
- `SizingSlotId`
- `PanelDefinitionId`
- `PanelInstanceId`

Ids use the bounded lowercase opaque-id grammar from `longhorn-core`.
Transport labels, array positions, titles, and product resource ids never
mint layout identity.

One document also carries:

- current schema version
- monotonic `LayoutRevision`
- bounded container, region, sizing-slot, and panel-instance counts

Overflow fails typed. Revision never wraps or resets inside one durable
document.

## Definition Registry

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
- one per container
- bounded per document and per container
- explicitly multiple within document limits

Missing policy fails registration. Empty allowed placement rejects every
placement. It never means unrestricted.

Definitions contain no title, icon, product attachment, serialized body, or
runtime handle.

## Durable State

A layout document contains:

- revision
- ordered layout containers
- schema id per container
- ordered panel-instance ids per region
- active panel-instance id per region
- collapse state only on supported regions
- current value per registered sizing slot
- panel instance id to definition id

Every panel instance appears in exactly one region of exactly one container.
Every referenced schema, region, sizing slot, definition, and instance must
exist. Durable state contains no derived visibility and no product payload.

## Normalization

Current-schema commands operate only on valid input. Corrupt or future state
enters the configuration recovery path instead of being silently repaired.
Explicit migrations may translate older documents before validation.

Successful mutation normalizes:

- containers in stable id order
- regions and sizing slots in registry order
- panel order exactly as committed
- active panel to the requested member, otherwise the first remaining member
- empty regions to no active panel

Closing or moving the active panel selects the panel now occupying its former
index. If the removed panel was last, the previous final panel becomes active.
An empty region has no active panel.

Unknown ids, duplicates, incomplete reorder permutations, invalid sizing
values, and policy violations reject the entire request.

## Visibility

Visibility is a projection:

- `keep-visible` regions remain visible when empty
- `hide-when-empty` regions are hidden when empty
- occupied regions are visible
- an optional transient reveal query may expose empty eligible regions during
  a drag

Transient reveal never changes the document or revision. g01.005 does not own
DOM drop zones or cross-window leases.

## Mutation Protocol

Every mutation request carries:

- bounded `LayoutRequestId`
- expected layout revision
- one command

Commands cover:

- create panel instance at a container, region, and insertion point
- close panel instance
- activate panel instance
- reorder one region using a complete permutation
- move panel instance across regions or containers
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

## Persistence

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

## Rust And TypeScript

Rust serde types are authoritative. Contract 010 applies:

- snapshots, commands, receipts, and errors generate checked TypeScript
- generated files live in `@inflatable-cookie/longhorn/layout`
- regeneration must be zero-diff
- unknown future variants fail explicit compatibility checks
- TypeScript cannot invent normalization, placement fallback, or active state

g01.005 supplies protocol types and framework-neutral helpers only. Tauri
transport, subscriptions, Svelte stores, and Poodle adapters remain later
packages.

## Composition

A no-Surface host binds:

```text
WindowId -> LayoutContainerId -> RegionId -> PanelInstanceId
```

A Surface host binds:

```text
WindowId -> SurfaceId -> LayoutContainerId -> RegionId -> PanelInstanceId
```

The layout document contains neither `WindowId` nor `SurfaceId`. Binding
lifecycle belongs to the host module. The same resolver and mutation engine
serve both shapes.

## Acceptance

- Loophole eight-region and Nucleus five-region fixtures share one engine
- Nucleus fixtures contain no Surface type or state
- Loophole fixtures adapt Surface ids only at the host binding edge
- singleton, one-per-container, bounded, and explicit-multiple policy pass
- missing policy and empty allowed placement fail closed
- every rejected mutation preserves the exact document and revision
- active selection after create, close, reorder, and move is deterministic
- sizing values are fixed-point, bounded, and cross-language stable
- empty-region visibility and transient reveal are deterministic projections
- layout and window domains survive concurrent independent mutation
- Rust and generated TypeScript fixtures round-trip exactly
- package graph contains no product, Poodle, Svelte, Tauri, or Surface
  dependency

## Implementation Evidence

Cards 023-027 implement this contract through:

- `longhorn-layout` for registration, resolution, state, normalization,
  visibility, and authoritative mutation
- `longhorn-layout-config` for registered persistence, registry-digest
  migration policy, fresh coordinated publication, debounce, and flush
- `longhorn-bindings` for checked Rust-to-TypeScript generation
- `@inflatable-cookie/longhorn/layout` for generated protocol types and exact framework-neutral
  helpers
- checked Loophole and Nucleus conformance fixtures under `fixtures/layout/`

Both conformance shapes use the same resolver and mutation engine. Surface and
window identities occur only in external host-binding fixture records.
Layout/window concurrency remains a separate-domain proof in
`longhorn-layout-config`.

Implementation does not claim donor migration. Contract 003 remains open
until consumer lanes remove superseded app-owned copies.

## Deferred

- host binding and optional Surface lifecycle: `g01.006`
- cross-window transfer: contract 011
- Svelte and Poodle lifecycle: contract 013 and `g01.007`
- consumer cutover: contract 003 and `g01.014` onward
- arbitrary recursive split trees: uncontracted
