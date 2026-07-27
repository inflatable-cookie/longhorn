# 011 Cross-window Transfer

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`

## Boundary

Longhorn owns an id-only transfer protocol and authoritative commit. Poodle
owns same-webview drag primitives. Consumers own target policy and whether
empty-display drops may create new hosting.

## Session

A host-created `DragSessionId` identifies one short-lived transfer. Its
record contains ids only:

- source window and layout container
- source region and panel or Surface id
- source layout revision
- allowed operation and instance policy
- expiry and unguessable nonce

Serialized panel, Surface, or product state never travels in a drag payload.

## Targets

Each renderer leases current drop-zone snapshots to the host:

- window, layout-container, and region ids
- rectangle in `ScreenDip`
- optional insertion hint
- accepted capability and policy
- layout revision and lease expiry

Zones are advisory. The host re-resolves domain state before commit.
Destination events may identify a zone directly; an outside-window drop may
instead provide a screen point for host hit-testing.

## Commit

The host validates:

- session and nonce
- source still exists at the expected authority
- target and region still exist
- current source and target revisions
- allowed region and instance policy
- consumer capability and hosting policy

One authoritative transaction moves or copies the item. Its result includes
the final revision and snapshot. Missing, stale, ineligible, expired, or
ambiguous targets abort without mutating the source.

Same-webview movement may use Poodle's HTML5 payload but invokes the same
authoritative mutation.

## Composition

- A no-Surface consumer targets a window-bound layout container.
- A full-hosting consumer targets a Surface-bound layout container.
- Panel drops on empty display space do not create a window by default.
- Surface drops may create a window only when the consumer explicitly enables
  that policy.

Optimistic renderer projection must reconcile to the authoritative response
and roll back on abort.

## Acceptance

- no transfer payload contains serialized durable model state
- stale source, stale target, disappeared window, and expired lease abort
  without source mutation
- Nucleus-shaped fixtures target windows without importing Surface types
- Loophole-shaped fixtures target Surfaces and retain allowed-region policy
- scale-boundary hit tests use `ScreenDip`
- optimistic UI converges to the returned authoritative revision
- packaged multi-window proof covers direct target and screen-point fallback

