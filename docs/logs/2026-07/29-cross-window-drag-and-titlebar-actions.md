# Cross-window Drag And Titlebar Actions

Date: 2026-07-29
Card: 040
State: complete

## Outcome

- added exact Longhorn native drag MIME and two-field payload checks
- armed panel transfer through Poodle's public pointer preparation seam
- added framework-neutral Surface drag and drop actions
- measured DOM targets into complete client-local replacement leases
- committed explicit leased zones and integer screen-DIP points
- projected compatible hidden regions without durable layout mutation
- added injected titlebar native drag with shared interactive exclusions
- kept Poodle visuals and private implementation details out of Longhorn

## Authority Boundary

The renderer may serialize only protocol version and a host-issued 128-bit
session id. It cannot invent fallback identity or carry panel, Surface,
layout, window, binding, or product state.

Lease actions measure `getBoundingClientRect()` and publish the full current
registry against the latest host-issued client id and epoch. The checked Tauri
handler remains the only client-to-screen geometry projector. Current bounds,
lease generation, target binding, layout revision, and eligibility are
revalidated at commit.

Same-window Poodle moves and cross-window commits both terminate at the
registered layout mutation authority. Compatible reveal is a
`RegionVisibility` projection. It dispatches nothing and disappears through
the consumer's terminal callback.

## Public Composition

- `@inflatable-cookie/longhorn-transfer`: strict payload parse and serialization
- `@inflatable-cookie/longhorn-svelte`: injected `windowDrag`
- `@inflatable-cookie/longhorn-svelte/transfer`: `DropZoneLeaseRegistry`
- `@inflatable-cookie/longhorn-svelte/surface-transfer`: armed Surface source and target actions
- `@inflatable-cookie/longhorn-poodle`: public drag props and compatible visibility projection
- `@inflatable-cookie/longhorn-poodle/transfer`: optional panel source and target factories

The Poodle root has no Longhorn transfer import. The optional transfer subpath
uses only public `DockExternalDragSource` and `DockExternalDropTarget` types.
No private selector, generated id, class, or Poodle MIME value appears.

## Evidence

| Risk | Proof |
| --- | --- |
| async dragstart race | mounted Poodle race writes no Longhorn payload and cancels the late session |
| payload authority | parser accepts only version and lowercase 128-bit session id |
| reveal mutation | Loophole matrix reveals only compatible `secondary`; document bytes do not change |
| geometry | lease fixture publishes exact local rectangles; host projection and stale-bounds Rust fixtures remain passing |
| epoch | action teardown republishes the reduced lease with the replacement client id and epoch |
| terminal cleanup | end, pre-drag race, Poodle unmount, lease action destroy, state stop, host expiry, and window destroy paths are covered |
| target resolution | mounted panel proof commits an explicit zone; Surface proof commits a screen point |
| titlebar | controls, links, roles, opt-outs, modifiers, and non-primary buttons are rejected; sync and async failures report |
| package boundary | root import audits stay Surface-free; optional transfer subpaths are explicit |

## Validation

- focused TypeScript, Svelte, package, transfer, Poodle, and Tauri-transfer
  checks passed
- full Effigy QA passed
- `effigy doctor` retains the known window-lifecycle god-file baseline; Card
  040 adds no high finding

## Current State

Card 040 is complete. Card 041 is ready and not started.

## Next

Start Card 041 three-shape app shell proof and closeout.
