# 045 Svelte Settings Session And Poodle Shell

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.008 batch 2
Governing refs: contracts 005, 012, and 013; research memo 012
Depends on: Card 044
Auto-start next card: no

## Objective

Add per-instance Svelte settings sessions and one public-Poodle shell that can
host consumer pages as a modal, window, or routed panel.

## Scope

- optional `@inflatable-cookie/longhorn-settings/svelte` and `/poodle` subpaths
- per-instance route, registry, page-session, draft, apply, reset, conflict,
  activation, and error state
- consumer renderer-key resolver and page snippets
- deterministic navigation, search results, and structural deep links
- staged dirty page-switch and close guards
- immediate pending/error state
- scoped Apply, Cancel, Reset, and confirmation surfaces
- modal, independent-window, and panel host composition
- exact Card 038 Poodle artifact and public primitives
- mounted accessibility, lifecycle, and teardown fixtures

## Public Behavior

Each shell owns its own route and page sessions. Page bodies register through
renderer keys and standard controllers; they keep product state and copy.

Staged dirty state blocks page switch or close until the user applies, discards,
or stays. Immediate changes show pending and failure but never claim saved
before authority receipt. Activation notices survive route changes until host
authority clears them.

The host form changes; registry, session, and authority behavior do not.

## Out Of Scope

- automatic product form generation
- product page copy or fields
- Poodle source changes
- backup/restore page implementation
- layout, Surface, command, or backend integration

## Steps

1. Add explicit optional Svelte and Poodle entry points and peers.
2. Bind checked clients to per-instance reactive registry and scope state.
3. Add renderer resolution and standard page-session controllers.
4. Add search, navigation, deep-link, and focus state.
5. Add staged dirty guards and immediate mutation status.
6. Add scoped Apply, Cancel, Reset, conflict, and activation presentation.
7. Compose modal, window, and panel hosts through public Poodle primitives.
8. Mount one-page, multi-page, missing-renderer, reconnecting, conflict, and
   recovery fixtures.
9. Audit accessibility, teardown, peer runtime, and package boundaries.

## Acceptance Criteria

- importing the root creates no singleton or Svelte/Poodle dependency
- two shell instances keep independent route and draft state
- optional modules have no empty navigation groups
- missing renderer fails before guarded shell reveal
- dirty staged state cannot disappear on page switch or close
- immediate failure never renders Saved
- stale conflict preserves the draft and exposes fresh authority
- activation state remains distinct from persistence success
- search and direct links focus the resolved page/anchor accessibly
- modal, window, and panel fixtures use one controller
- repeated mount/unmount leaves no listener, timer, or pending controller work
- only public Poodle APIs and the exact artifact set are used

## Evidence Required

- session state and close-guard matrix
- two-instance isolation fixture
- mounted modal/window/panel fixtures
- loading, unsupported, reconnecting, conflict, recovery, and failure states
- keyboard/focus/accessibility report
- Poodle artifact and public-API audit
- package, peer, SSR, and teardown checks
- Svelte, TypeScript, and Effigy QA

## Stop Conditions

- Poodle lacks a required public dialog/navigation/focus seam
- one mandatory app frame is required
- page bodies must move product authority into Longhorn
- host forms need different session semantics
- mounted teardown cannot cancel pending work exactly

## Next Task

Card 046 is ready. Add checked storage/profile/backup clients and optional
shared pages without weakening contract-004 authority or receipts.

## Result

`@inflatable-cookie/longhorn-settings/svelte` now provides isolated per-instance registry,
scope, route, draft, mutation, guard, recovery, conflict, and activation
state. Renderer keys resolve before reveal. Scope and listener connections
start listener-first, reconnect explicitly, ignore late work after stop, and
tear down exactly.

`@inflatable-cookie/longhorn-settings/poodle` adds one controller-driven shell for modal,
independent-window, and routed-panel hosts. It uses public Poodle primitives
from the exact Card 038 artifact. Consumer snippets retain product schemas,
copy, validation, and intent codecs.

Focused evidence covers two-instance isolation, dirty navigation and close,
single-unit Apply limits, immediate failure, stale conflict, recovery,
activation, missing renderers, reconnect, late work, remount teardown, all
three host forms, structural search focus, SSR imports, optional peer
boundaries, and package contents.

Evidence:
`../../../logs/2026-07/29-svelte-settings-session-and-poodle-shell.md`.
