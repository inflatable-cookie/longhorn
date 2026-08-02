# 109 Loophole Renderer, Poodle, And Transfer Cutover

Status: complete
Owner: Tom
Roadmap: g01.015 batch 3
Governing refs: contracts 010-014; Cards 107-108
Depends on: Card 108
Auto-start next card: no

## Objective

Replace renderer snapshot authority and ad hoc drag shaping with checked
per-window clients and Poodle's public external drag seam.

## Repository Scope

- Longhorn: admitted client/transfer fixes and mounted fixtures only.
- Loophole: Aura renderer bindings, drag adapters, capabilities, tests, and docs.
- Poodle: read-only exact public-source and artifact verification.

## Scope

- listener-first layout and Surface sessions with authority epochs
- bounded optimism and exact stale-result rejection
- public Poodle controlled region/tab composition
- same-region reorder and same-window cross-region move
- armed cross-window panel sessions, complete leases, and target revalidation
- whole-Surface drag composition and titlebar behavior
- multi-window teardown, remount, and client-epoch replacement

## Steps

1. Bind current panel bodies and shell chrome to controlled Longhorn projections.
2. Replace private/local drag payload shaping with Poodle external hooks.
3. Arm host sessions before dragstart and publish complete measured leases.
4. Commit panel movement through the same registered layout authority.
5. Reconcile optimism to authoritative receipts and roll back aborts.
6. Cancel sessions on end, unmount, supersession, epoch change, and window destroy.
7. Remove renderer-authored durable snapshots and raw Longhorn invoke/listen calls.
8. Prove two-window movement, ambiguity, expiry, disappearance, remount, and 2x geometry.

## Acceptance Criteria

- native payload contains only protocol version and host-created session id
- no Poodle MIME, DOM, class, portal, or internal id becomes Longhorn knowledge
- hidden eligible regions reveal only as projection during drag
- cross-document panel attempts fail before mutation
- stale renderer results cannot overwrite current authority
- panel catalogue and presentation remain Aura-owned

## Stop Conditions

- preparation cannot exist before native dragstart
- target geometry cannot be projected to checked screen DIPs
- product target policy would move into Poodle or Longhorn

## Evidence Required

- mounted two-window session and teardown traces
- same-window and cross-window transfer receipts
- expiry, ambiguity, disappearance, stale-result, and 2x geometry failures
- public-Poodle and raw-transport audits

## Result

- Aura now installs panel and whole-Surface adapters over one shared Tauri
  transfer handler, renderer client epoch, and measured lease registry.
- All eight regions use Poodle's public `externalDragSource` and
  `externalDropTarget` contracts. The former app MIME, tab-id parsing,
  Poodle-class lookup, capture drag orchestration, and drag-out heuristic are
  removed.
- The renderer arms sessions on pointer preparation, publishes panel-region
  and Surface-window zones, rejects stale layout and Surface revisions, and
  destroys leases and sessions on remount or unmount.
- Same-window reorder stays Poodle-controlled. Cross-region and cross-window
  panel moves publish through the registered layout authority. Whole-Surface
  managed-window moves publish through the registered Surface authority.
- Empty-display creation remains Loophole policy. It runs only after the
  checked Surface terminal result explicitly reports disabled generic
  provisioning, then uses the retained Loophole spawn path.
- Compatibility projection preserves Aura panel priority and disables an
  emptied source window without moving catalogue, focused-Surface, or native
  window policy into Longhorn.

## Validation

- Loophole Svelte diagnostics: 0 errors, 0 warnings.
- Loophole renderer: 1,021 passed across 101 files; focused transfer and policy
  set: 13 passed.
- Loophole production renderer build passed; only existing Vite chunk-size and
  mixed static/dynamic import warnings remain.
- Loophole native compatibility sync: 2 passed; Rust library check passed.
- Longhorn transfer protocol: 11 passed; Surface transfer protocol: 6 passed;
  Svelte transfer lifecycle/actions: 20 passed; Tauri transfer handler and
  capability set: 7 passed.
- Shared Tauri transfer evidence covers epoch replacement, teardown,
  ambiguity, expiry, disappearance, stale geometry, and scaled projection.
- Aura's source boundary test rejects private Poodle MIME, DOM, class, and
  internal-id discovery.

## Next Task

Execute Card 110's settings, command, palette, and keyboard cutover.
