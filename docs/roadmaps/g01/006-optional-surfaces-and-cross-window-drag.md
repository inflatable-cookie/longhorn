# g01.006 Optional Surfaces And Cross-window Drag

Status: complete
Owner: Tom
Updated: 2026-07-29
Governing refs: contracts 001-004 and 009-014; research memos 003 and 010

## Outcome

Add Loophole's full window-to-Surface composition as an optional module and
prove host-authoritative cross-window transfer for direct-window and
Surface-hosted layouts.

## Generation Runway Contribution

This milestone completes the optional hosting and transfer layer above the
delivered display, window, and layout foundations. It leaves framework-neutral
Rust and TypeScript clients for g01.007, then unblocks the Nucleus and Loophole
consumer lanes without making Surface state mandatory.

## Goals

- [x] implement optional bounded Surface identity, topology, and resolution
- [x] implement expected-revision Surface lifecycle and registered persistence
- [x] compose full Surface hosting through the existing window host
- [x] implement bounded host-created transfer sessions and drop-zone leases
- [x] commit same-document cross-window panel moves authoritatively
- [x] commit whole-Surface moves with explicit window-provision policy
- [x] generate checked framework-neutral Surface and transfer clients
- [x] prove both composition shapes in a packaged multi-window app
- [x] preserve deferred cross-document, copy, UI, migration, and platform work

## Execution Plan

### Batch 1: Surface domain

- [x] Card 028 — Surface identity, topology, presence input, and resolution
- [x] Card 029 — expected-revision lifecycle and registered persistence

### Batch 2: Host composition

- [x] Card 030 — Surface/window host composition and two-shape conformance
- [x] reassess Contract 011 only if the delivered Surface binding, revision,
  or persistence boundary changed

### Batch 3: Transfer targeting

- [x] Card 031 — bounded sessions, complete replacement leases, expiry,
  cancellation, and deterministic target resolution

### Batch 4: Authoritative commits

- [x] Card 032 — same-document panel transfer through existing layout mutation
- [x] Card 033 — whole-Surface transfer and opt-in window provisioning

### Batch 5: Protocol and packaged proof

- [x] Card 034 — generated TypeScript protocol and Tauri host assembly
- [x] Card 035 — packaged multi-window proof, boundary audit, and closeout

## Authority Decisions

### Optional Surface state

`longhorn-surfaces` owns Surface ids, external layout-container bindings,
hosting preferences, ordered window membership, active Surface, resolution,
and lifecycle. `longhorn-surfaces-config` persists this as a distinct
registered domain.

Consumers own product presence predicates, window roles, layout-container
seeding and cleanup, native factory policy, and product resources. Nucleus
does not import Surface packages.

### Transfer sessions

`longhorn-transfer` owns process-local bounded sessions, leases, cancellation,
and target resolution. Payloads contain only protocol version and an
unguessable session id. Enumeration order never resolves overlapping targets.

### Panel commits

The first line supports move only. Source and target must share one registered
layout document. The existing expected-revision `MovePanel` command performs
the only durable commit. Cross-document transfer fails before publication.

### Surface commits

Whole-Surface transfer changes only the Surface document and retains the
layout-container binding. Empty-display window creation is opt-in through an
injected provisioner with explicit cleanup receipts.

## Acceptance Criteria

- [x] Nucleus-shaped dependencies import no Surface state
- [x] Loophole-shaped fixtures retain full window-to-Surface-to-layout hosting
- [x] every Surface binds one distinct layout container and resolves to at
  most one window
- [x] product presence predicates remain consumer-owned
- [x] stale or rejected Surface lifecycle preserves exact state
- [x] drag payloads contain no durable subject or product state
- [x] session and lease registries are finite, expiring, and single-use
- [x] ambiguous, stale, expired, replayed, or disappeared targets abort
- [x] direct and Surface target bindings share the target-resolution core
- [x] panel move publishes exactly one registered layout document
- [x] cross-document and copy transfer fail before mutation
- [x] whole-Surface movement retains its layout-container binding
- [x] new-window provisioning is explicit and cleanup is receipted
- [x] Rust-generated TypeScript fixtures round-trip exactly
- [x] packaged proof covers real multi-window success, cancellation, expiry,
  target loss, overlap, and scale boundaries
- [x] Rust 1.85 and full Effigy QA pass

## Planning Gaps Kept Visible

- cross-document atomic layout mutation has no promoted contract
- panel copy is not proven by donors
- reusable Svelte and Poodle drag adapters belong to g01.007
- Loophole and Nucleus ownership transfer belongs to g01.014-g01.015
- non-macOS packaged transfer behavior remains platform evidence, not an
  inferred guarantee

These gaps remain owned by later milestones.

## Planning Checkpoint

Card 030 closed the Surface foundation checkpoint. Cards 028-030 preserved the
promoted binding, revision, and persistence boundary. Contract 011 remains
current. Card 031 now supplies the shared bounded session, lease, and target
core. Card 032 now binds consumed attempts to fresh opaque host bindings and
the existing same-document expected-revision `MovePanel` publication. Card
033 adds fresh whole-Surface admission, ordinary and empty-display target
re-resolution, expected-revision `MoveSurface` publication, exact
layout-binding retention, and explicit provision, cleanup, partial-failure,
and reconciliation evidence. Card 034 now supplies checked Surface, transfer,
and optional Surface-transfer protocols, framework-neutral clients, one
epoch-safe renderer connection, narrow Tauri transport and handler assembly,
managed-window geometry projection, optional Surface commands, and minimal
capability matrices. Card 035 adds separate direct and Surface-enabled Rust
1.85 packaged artifacts, real multi-webview commands, authoritative panel and
Surface commits, explicit empty-display provision, failure invariance,
2× scale and half-open boundary evidence, and dependency, payload, capability,
and authority audits. `g01.006` is complete.

## Next Task

Start Card 036 from the compiled g01.007 runway.
