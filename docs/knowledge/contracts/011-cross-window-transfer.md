# 011 Cross-window Transfer

Status: active compiled boundary
Owner: Tom
Updated: 2026-11-22
Depends on: contracts 002, 004, 009, 010, and 014
Evidence: `010-surface-hosting-and-transfer-boundary.md` (Git history),
`011-client-svelte-poodle-and-shell-boundary.md` (Git history)

## Boundary

Longhorn owns bounded host-created transfer sessions, leased drop-zone
snapshots, deterministic target selection, and authoritative adapter commits.
Poodle owns same-webview drag primitives. Consumers own target capability
policy and optional empty-display window provisioning.

Loophole proves whole-Surface movement between windows and optional window
creation on an empty display. Cross-window panel transfer is a new capability.
The first implementation therefore shares session and target mechanics but
keeps panel and Surface commit adapters explicit.

## Package Boundary

- `longhorn-transfer` owns session, lease, target, and panel-transfer
  coordination using only the shared layout/document types from
  `longhorn-surfaces`, without importing whole-Surface transfer policy or
  provisioning from `longhorn-surface-transfer`.
- `longhorn-surface-transfer` supplies the optional whole-Surface transfer
  adapter.
- `longhorn-tauri-transfer` binds current managed-window readback and handler
  assembly without owning product policy.
- `@inflatable-cookie/longhorn/transfer`, `@inflatable-cookie/longhorn/surfaces`, and
  `@inflatable-cookie/longhorn/surface-transfer` contain checked protocol types and
  framework-neutral clients.

Nucleus may depend on `longhorn-transfer` without linking
`longhorn-surface-transfer`.

## Session

A host-created `DragSessionId` identifies one bounded process-local transfer.
Ids come from an injected cryptographically strong allocator and carry at
least 128 bits of entropy. A session is:

- finite and bounded by host policy
- short-lived under an injected monotonic clock
- single-use for commit
- explicitly cancellable
- never persisted
- bound to one initiating client and source window

At session creation, the host re-resolves and records ids and authority
evidence only:

- transfer subject kind and id
- source window and host binding
- source layout or Surface document identity
- source container and region when the subject is a panel
- current source revision
- allowed target capability
- expiry

The renderer receives a protocol version and `DragSessionId`. Serialized
panel, Surface, layout, product, or resource state never enters the drag
payload. A renderer-supplied source snapshot is not authority.

Capacity exhaustion, entropy failure, unknown source, ineligible subject, or
invalid lifetime rejects session creation without allocating a partial record.

`longhorn-transfer` represents `DragSessionId` as exactly 128 allocator-supplied
bits and serializes it as 32 lowercase hexadecimal bytes. The package supplies
no random or wall-clock implementation. Consumers inject both the
cryptographically strong allocator and monotonic clock.

## UI Session Arming

The host-created session must exist before a native `dragstart` handler writes
the payload. A renderer cannot await session creation and then mutate
`DataTransfer` after that handler returns.

The reusable UI adapter therefore arms a session from an explicit pre-drag
gesture and binds the prepared result to the exact subject, source binding,
client epoch, and pointer sequence. If preparation has not completed when
native drag starts, the cross-window gesture is rejected. It does not emit a
renderer-authored fallback payload.

Unused, superseded, ended, unmounted, or window-destroyed preparations cancel
their host session. Same-window Poodle reorder may remain local, but any
gesture advertised as cross-window uses the armed Longhorn session.

## Drop-zone Leases

Each renderer publishes a complete replacement lease for one current window
and client epoch. A lease contains a bounded set of unique `DropZoneId`
records:

- current `WindowId`
- target host binding (whose target variant — panel region or Surface window —
  determines the derived accepted capability)
- target `SurfaceId` and `RegionId` for panel-region targets
- rectangle in `ScreenDip`
- optional bounded insertion position
- target layout or Surface revision
- lease generation and expiry

Zones are advisory. They contain no durable model state. Unknown,
zero-extent, overflowing, outside-window, duplicate, or over-limit geometry
fails the complete lease.

The host checks leased rectangles against fresh managed-window outer bounds.
Renderer client geometry reaches `ScreenDip` only through an explicit checked
projection at the host boundary. A renderer cannot relabel its lease as
another window.

Publishing a newer generation replaces the prior lease atomically. Window
destroy, client-epoch change, or expiry invalidates it. Drop zones are not a
durable event stream.

The host explicitly binds one current client and monotonically advancing epoch
to each managed window. Epoch change removes the prior lease, resets the
epoch's accepted lease generation watermark, and invalidates active source
sessions from the superseded client; a client snapshot therefore reports only
the protocol version, client identity, and client epoch. Within one client
epoch, lease generation monotonicity is preserved across lease expiry and
reclamation: re-publishing an equal or lower generation after expiry still
fails with `stale_lease_generation`. Session, current-client, lease, per-lease
zone, insertion-position, and lifetime limits are finite and checked before
publication; expired leases are discarded when reclaimed on lease publication
so they do not hold lease capacity. Before reclamation, a terminal resolution
attempt against an expired lease reports `lease_expired`; once reclaimed (for
example by an unrelated window's lease publication), no expired lease record is
retained, so resolution reports `no_target` unless another live target matches.

## Target Resolution

A commit request names the session and either:

- one leased `DropZoneId`, or
- one `ScreenPoint` for host hit-testing

An explicit zone still requires a current lease and fresh authority checks.
Screen-point resolution uses current managed-window outer bounds, then current
zones for that window. No match, multiple containing windows, multiple
eligible zones, stale geometry, or disappeared hosts aborts as typed
ambiguity or absence. Array order and renderer focus do not break ties.

Panel drops on empty display space never create a window in the first protocol
line. Whole-Surface drops may ask an injected consumer provisioner for a
window only when explicit policy enables it. Provisioning failure cannot
remove the source Surface.

## Panel Commit

The first panel-transfer line supports move only. Copy is unproven and
deferred.

Before commit, the host reloads:

- session and source binding
- target binding and current window presence
- source and target layout revisions
- panel existence and movability
- target region existence and placement eligibility
- instance policy and target capability

Source and target must belong to the same `RegisteredSurfaceDomain`. The
existing expected-revision `MovePanel` command performs one coordinated
publication and returns the authoritative layout snapshot.

Cross-document panel transfer is explicitly unsupported in this line. It
fails before mutation. A future multi-domain transaction contract must land
before that restriction changes.

Same-webview Poodle movement may bypass leased screen geometry, but it invokes
the same authoritative layout mutation.

## Surface Commit

A whole-Surface transfer re-resolves:

- the Surface still exists at the recorded revision
- the source window still hosts it
- the target window still participates and may accept it
- consumer move and empty-window policies

The expected-revision Surface move publishes one complete unified Surface
document or none. Relocation preserves the Surface identity and its contained
regions, panels, and sizing slots while updating host placement.

If an empty-display policy provisions a window before the Surface commit, every
configured empty-display target window must appear in `allowed_target_windows`
(validated when constructing `SurfaceTransferPolicy`), and the provisioner
returns explicit commit and cleanup authority. A failed Surface commit must
invoke cleanup and report both receipts. Unresolved cleanup or post-commit host
failure is a typed host-reconciliation failure (`reconciliation_required`),
never silent success.

## Completion And Cancellation

The first terminal commit attempt consumes the session whether it succeeds or
aborts. Cancellation is idempotent and consumes the session. Expiry and host
shutdown discard sessions and leases without durable mutation.

Success returns:

- request and session ids
- subject kind
- source and target bindings
- previous and committed revision
- authoritative domain snapshot
- target-resolution evidence
- optional provisioning and cleanup receipt

Abort returns a stable code (promoting underlying transfer-domain rejections
and preserving layout/Surface mutation rejection codes) and current authority
evidence when safe. Transfer aborts are terminal for the request
(`retryable: false`): callers must acquire fresh authority or start a new
session rather than replaying a failed request. Missing, stale, ineligible,
expired, ambiguous, replayed, cancelled, cross-document, or disappeared targets
leave the source document byte-for-byte unchanged.

Optimistic renderer projection must reconcile to the authoritative response
and roll back on abort. Optimism never advances durable revision locally.

The pure coordinator marks the first active terminal target-resolution attempt
consumed before checking current target evidence. Target absence, stale
geometry, lease expiry, or ambiguity therefore cannot be retried under the
same session. Cancellation remains separately idempotent.

## Limits

The first protocol line does not claim:

- cross-document panel transactions
- copy transfer
- keyboard drag presentation
- non-Tauri native hosts
- automatic panel-to-new-window creation
- durable recovery of in-flight sessions

These require separate contract evidence.

## Implementation Evidence

Card 031 implements the process-local session, lease, and deterministic target
core in `longhorn-transfer`. Card 032 adds fresh movable-panel admission,
opaque direct-window and Surface-container host projections, same-domain and
revision rechecks, and the existing coordinated expected-revision `MovePanel`
publication.

The package now depends on core, config, surfaces, surfaces-config, and serde
(using the shared layout/document types folded into `longhorn-surfaces`); it
imports no whole-Surface transfer adapter or provisioning types. Cross-document
and copy attempts consume the terminal session and publish no bytes. Success
returns the existing authoritative layout and configuration receipt.

Card 033 adds `longhorn-surface-transfer`. Admission resolves the fresh
registered Surface and primary host binding. Terminal commit rechecks the
recorded revision, current primary host, target participation, declared host
preference, consumer policy, insertion, and empty-window policy before using
the existing expected-revision `MoveSurface` publication.

The transfer core preserves a distinct empty-display terminal result only for
a screen point outside all fresh managed-window bounds. The optional adapter
then requires consumer-supplied display bounds, logical window, placement,
and target policy before calling an injected provisioner. Provision returns a
hidden, placed, ready receipt and retained authority. Publication failure
invokes cleanup; unresolved cleanup or post-publication host commit failure
returns typed reconciliation evidence.

Card 034 adds Rust-generated `@inflatable-cookie/longhorn/surfaces`, `@inflatable-cookie/longhorn/transfer`, and
`@inflatable-cookie/longhorn/surface-transfer` protocols and clients. `@inflatable-cookie/longhorn-tauri` is the
sole raw invoke/listen transport. `longhorn-tauri-transfer` binds commands to
the caller's current managed-window identity, projects checked client
geometry, and exposes optional Surface commands over the same coordinator.
The client installs the epoch listener before requesting its initial snapshot,
accepts only the newest epoch, and tears down safely even when listener
registration resolves late.

Packaged multi-window runtime evidence remains Card 035.

## Acceptance

- drag payloads contain only protocol version and `DragSessionId`
- session and lease stores are finite, expiring, single-use, and clock-driven
- stale source, stale target, disappeared window, expired lease, replay, and
  ambiguous overlap abort without source mutation
- invalid lease replacement preserves the prior valid generation
- explicit-zone and screen-point resolution produce the same eligible target
- Nucleus-shaped fixtures target direct window containers without Surface
  types
- Loophole-shaped fixtures target Surface containers and retain placement
  policy
- panel move commits through one registered layout document
- cross-document panel move fails before publication
- whole-Surface movement retains its layout-container binding
- empty-display provisioning is opt-in and cleanup is receipted
- scale-boundary hit tests use checked `ScreenDip`
- Rust-generated TypeScript fixtures round-trip exactly
- packaged multi-window proof covers success, cancellation, lease expiry,
  target disappearance, and ambiguous overlap
