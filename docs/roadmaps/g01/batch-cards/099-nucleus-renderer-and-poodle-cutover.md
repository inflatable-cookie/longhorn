# 099 Nucleus Renderer And Poodle Cutover

Status: completed
Owner: Tom
Roadmap: g01.014 batch 3
Governing refs: contracts 003, 010, 012-014, and 017; Poodle overlay geometry
boundary; Cards 094-095 and 098
Depends on: Card 098
Auto-start next card: no

## Objective

Adopt the checked layout client, per-window Svelte lifetime, and public Poodle
bindings without turning Longhorn into the Nucleus app shell.

## Repository Scope

- Nucleus renderer, dependency lock, and focused tests may change.
- Longhorn may receive mounted conformance fixtures and migration evidence.
- Poodle changes require a separate upstream card.

## Scope

- checked generated layout protocol
- listener-before-snapshot connection and revision freshness
- per-window Svelte state and request-keyed reconciliation
- public Poodle Tabs, DockRegion, and SplitView bindings
- consumer panel body, label, icon, resource, and frame resolvers
- project-switch, mount, unmount, drag, overlay, and teardown lifetime
- explicit final native-content visibility input
- removal of renderer whole-snapshot authority and private Poodle inspection

## Steps

1. Pin the exact private renderer and public Poodle source/artifact graph.
2. Connect one per-window layout state listener before its first snapshot.
3. Replace whole-snapshot persistence with revisioned command dispatch.
4. Bind regions, tabs, and sizing through public controlled Poodle APIs.
5. Keep the Nucleus frame and panel presentation in consumer resolvers.
6. Replace overlay DOM inspection with explicit final-visibility inputs.
7. Exercise project switching, drag, overlays, remount, and failure states.
8. Remove obsolete renderer state authority and private selector code.
9. Audit peers, lifecycle teardown, retained policy, and rollback.

## Acceptance Criteria

- renderer state is projection only and cannot persist an unversioned snapshot
- late loads, events, and mutation receipts cannot cross project or client epoch
- current panel create, close, activate, reorder, move, and sizing UX remains
- public Poodle APIs own presentation and drag semantics
- no Poodle class, generated id, private MIME, or source alias is inspected
- native-content final visibility is explicit and preserves overlay intersection
- Popover and Menu snapshot maps recompute intersection when either overlay or
  Browser viewport geometry changes
- loading, unsupported, reconnecting, and failure states remain visible
- teardown releases listeners, observers, timers, optimism, and drag state
- one exact private Svelte/Poodle graph resolves

## Evidence Required

- [x] mounted project-switch and mutation traces
- [x] listener, stale-result, remount, and teardown tests
- [x] Poodle public-API audit
- [x] dependency and peer-runtime inventory
- [x] renderer authority and private-selector diff
- [x] focused Svelte and Nucleus desktop checks

## Completion

Nucleus commit `74ca4e7c72f447e064419de6dc72502265cbbf49`
replaces aggregate workspace snapshot mutation with exact checked commands,
one project-scoped Svelte session, request-keyed optimism, public Longhorn
Poodle composition, and explicit Popover/Menu-to-Browser geometry.

Longhorn commit `ca755cbd332260abd971d86509f6190a0e76d269`
adds the state-only binding entry point, presentation pass-through, and
projection-only hidden SplitView panes. Nucleus retains project scope, panel
bodies and presentation, resources, frame composition, native handles, and
cleanup. No private selector, Poodle MIME, generated tab id, Surface package,
or unversioned renderer save remains.

Mounted tests prove project switch, late load, late mutation, listener
teardown, optimism cleanup, and remount. Overlay tests recompute final Browser
visibility when either surface or viewport geometry moves. The frozen private
file graph installs without a second Svelte runtime. Package-manager
publication remains deferred.

## Stop Conditions

- preserving overlay behavior requires a private Poodle selector
- the exact Poodle source or artifact lacks the compatible public seam
- renderer must fabricate a durable fallback snapshot
- one mandatory shared app frame becomes necessary
- donor worktree changes overlap renderer layout files

## Next Task

Execute Card 100. Use the explicit final visibility and checked lifetime to
cut the Browser child over to native-content coordination.
