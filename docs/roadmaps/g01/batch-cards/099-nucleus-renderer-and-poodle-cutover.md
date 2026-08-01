# 099 Nucleus Renderer And Poodle Cutover

Status: planned
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

- mounted project-switch and mutation traces
- listener, stale-result, remount, and teardown tests
- Poodle public-API audit
- dependency and peer-runtime inventory
- renderer authority and private-selector diff
- focused Svelte and Nucleus desktop checks

## Stop Conditions

- preserving overlay behavior requires a private Poodle selector
- the exact Poodle source or artifact lacks the compatible public seam
- renderer must fabricate a durable fallback snapshot
- one mandatory shared app frame becomes necessary
- donor worktree changes overlap renderer layout files

## Next Task

Execute Card 100. Use the explicit final visibility and checked lifetime to
cut the Browser child over to native-content coordination.
