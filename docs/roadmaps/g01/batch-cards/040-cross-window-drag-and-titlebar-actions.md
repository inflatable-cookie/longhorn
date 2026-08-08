# 040 Cross-window Drag And Titlebar Actions

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.007 batch 3
Governing refs: contracts 009-013; research memos 010 and 011
Depends on: Cards 037-039
Auto-start next card: no

## Objective

Add reusable armed cross-window transfer, compatible-region reveal, drop-zone
lease, and titlebar drag actions without owning Poodle visuals or product
policy.

## Scope

- Svelte transfer actions and Poodle public drag extension binding
- pre-drag panel and Surface session arming
- protocol-only native drag payload
- compatible hidden-region reveal projection
- complete drop-zone lease publication
- explicit-zone and screen-point commit
- cancel, expiry, end, unmount, and destroyed-window cleanup
- exact shared titlebar drag action
- injected native drag and error reporter

## Public Behavior

A cross-window gesture starts only with a prepared host session bound to the
current subject and client epoch. Drag payloads contain only protocol version
and session id. Eligible regions may reveal as presentation state; they do not
alter layout before authoritative commit.

Lease updates are complete replacements. Drop, cancellation, failure, and
teardown clear reveal state and reconcile to current authority.

The titlebar action accepts primary unmodified gestures outside interactive or
opt-out descendants, then calls an injected native drag function.

## Out Of Scope

- keyboard drag UX
- copy or cross-document transfer
- panel-to-new-window creation
- product eligibility policy
- visual drop-zone or titlebar components
- direct `@tauri-apps/api` import outside the raw transport edge

## Steps

1. Bind Card 038's preparation lifecycle to checked transfer clients.
2. Reject dragstart when the exact prepared session is absent or stale.
3. Encode only the shared version and session id.
4. Project compatible hidden regions from current layout policy.
5. Measure public target elements and publish complete checked leases.
6. Commit explicit zones and screen points through authoritative clients.
7. Reconcile success and every terminal abort.
8. Cancel unused or superseded sessions and release leases on teardown.
9. Add the injected titlebar action from the duplicated donor behavior.
10. Exercise race, geometry, cleanup, and interaction fixtures.

## Acceptance Criteria

- no async mutation of DataTransfer after dragstart
- no renderer-created fallback session id
- payload contains no panel, Surface, layout, or product state
- revealed regions remain projection-only
- stale geometry or client epoch cannot commit
- same-window and cross-window moves reach the same layout mutation authority
- end, cancel, expiry, unmount, and destroy clear every session and lease
- titlebar rejects buttons, links, inputs, controls, opt-outs, modifiers, and
  non-primary buttons
- failures use the injected reporter
- no Poodle-private selector, id, class, or MIME value is referenced

## Evidence Required

- pre-drag race and payload audit
- compatible-region reveal matrix
- lease replacement and geometry fixtures
- terminal cleanup matrix
- titlebar donor-equivalence fixture
- mounted multi-window mock proof
- package, TypeScript, Svelte, and Effigy QA

## Stop Conditions

- native drag must begin before the host session is ready
- Poodle public extension cannot express the lifecycle
- a renderer payload needs durable subject state
- geometry bypasses checked host projection
- cleanup cannot be made exact

## Next Task

Card 041 is ready but not started. It composes minimal and full shells from
produced artifacts and closes g01.007.

## Result

Longhorn now supplies strict two-field native drag payloads, prepared panel and
Surface sources, explicit-zone and screen-point targets, complete measured
drop-zone lease replacement, projection-only compatible-region reveal, and an
injected titlebar drag action.

Panel transfer uses Poodle's public extension types through the optional
`@inflatable-cookie/longhorn-poodle/transfer` subpath. The Poodle root remains Surface- and
transfer-free. Geometry stays caller-local until the checked Tauri host
projects it. Terminal callbacks clear consumer reveal state; end, race,
unmount, action destroy, state stop, epoch replacement, and host expiry retain
one cancellation and lease authority.

Evidence:
`../../../logs/2026-07/29-cross-window-drag-and-titlebar-actions.md`.
