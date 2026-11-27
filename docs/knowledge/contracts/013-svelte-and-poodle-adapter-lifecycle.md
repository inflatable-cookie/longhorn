# 013 Svelte And Poodle Adapter Lifecycle

Status: active compiled boundary
Owner: Tom
Updated: 2026-10-07
Evidence: `003-foundation-boundary-characterization.md` (Git history),
`011-client-svelte-poodle-and-shell-boundary.md` (Git history),
`014-command-input-and-palette-boundary.md` (Git history)

## Boundary

Framework-neutral clients own protocol state. Svelte adapters own reactive
lifetime. Poodle adapters bind Longhorn state and commands to public visual
primitives without copying component source or durable state.

## Client Lifetime

- Stores are created per app or window unless the consumer explicitly shares
  one. Importing a module does not create a hidden singleton.
- Module import is safe without `window` or a running Tauri host.
- Start attaches listeners before querying the current snapshot.
- Stop is idempotent and handles listener-registration promises that resolve
  after teardown. A start queued behind stop runs only if it is still current
  when teardown finishes. A later stop or destroy supersedes that start.
- Window destruction releases listeners, timers, drag leases, and pending
  optimistic projections.
- Transport and scheduler dependencies are injectable for tests.
- A shared lifecycle primitive may own registration, pump, and teardown. Each
  domain still supplies validation and freshness comparison.
- A client is added only for a real host command/event contract. Checked
  generated layout data does not imply an invented layout service.

## Reactive State

- Epoch and revision determine snapshot freshness.
- Renderer projections never become durable fallback truth. History, fork
  history, notification, and operation Svelte sessions expose read-only
  projections. Use session commands to change selection, filters, or pages.
- Surface mutation feedback retains pending requests and the 32 most recently
  settled operations within the session. Stop clears it.
- Optimistic mutations are keyed by request id, reconciled to the host result,
  and rolled back or resynced on failure. Layout stop clears the projection.
  An earlier dispatch can still return its outcome to its caller, but it
  cannot change the stopped or restarted projection.
- A settings mutation keeps its unit pending until settlement. A staged edit
  made during that request stays dirty for the next apply. Mutation snapshots
  follow the same registry generation, scope revision, and authority token
  checks as subscription snapshots.
- Errors, loading, reconnecting, and unsupported capability states are
  explicit.
- A shell action that fails (apply, guard resolution, reset) stays visible
  on the page and in any open confirmation until the next action. A session
  whose stop fails on unmount reports through the caller’s reporter or logs
  the error when none was supplied.
- Initial configuration read failures show one error with Retry. A loaded
  snapshot with an absent capability can show that capability as unavailable.
- Drop-zone lease geometry is refreshed on resize and on window or ancestor
  scroll. For position changes without those events, the layout owner can
  request publication after layout settles.
- An optional capability lives behind its package subpath. Importing the
  Surface-free root cannot resolve that capability.

## Poodle Boundary

- Adapters use public controlled props, snippets, and events.
- Poodle owns tabs, dock regions, split views, menus, dialogs, and visual
  semantics.
- Longhorn owns ids, eligibility, command dispatch, and authoritative state.
- Consumer panel bodies and product presentation remain slots or snippets.
- No adapter imports Poodle internals or copies Poodle source.
- Poodle's `WorkspaceLayoutSnapshot` is presentation state, not a second
  durable Longhorn document.
- Command palette and keybinding adapters project the sealed command registry,
  effective keymap, and current availability through public controlled Poodle
  APIs. Poodle owns focus and visual semantics; Longhorn owns ids, search,
  conflict records, and injected dispatch. The palette and keybinding
  settings share the matching rules in [contract 006](006-command-action-and-input.md#projection-and-ui).
  Keybinding reset requires confirmation. The palette shows search and
  execution failures while it stays open.
- A command selection calls a consumer executor. No Svelte or Poodle adapter
  invents a generic Tauri execution route.
- Same-window drag remains Poodle-owned. Cross-window binding requires public
  drag start, end, eligibility, and external-drop extension points.
- Poodle `g12.016` supplies those public points through typed asynchronous
  prepare, synchronous start, terminal end/cancel, target eligibility, and
  accepted-drop callbacks. Pending preparation emits no external payload.
- Private MIME values, generated DOM ids, CSS classes, and event-target
  reverse engineering are not public integration seams.

The titlebar-drag helper rejects interactive descendants (buttons, links,
inputs, selects, labels, summaries, editable content, and the button, link,
tab, and menu-item roles) and modified pointer gestures before asking the host
to start native dragging. Failure goes through an injected reporter.

## Shell Composition

- Theme and presentation bootstrap uses public Poodle token and provider APIs.
- Readiness reveal is explicit and follows authoritative load.
- Missing capability and host errors remain visible states.
- Guidance composes these pieces; Longhorn does not ship one mandatory app
  frame.

## Compatibility

- Svelte, Poodle, and Tauri compatibility ranges are declared peers.
- The Poodle adapter proof pins one exact published Poodle version. The
  preview-artifact era ended when Poodle released publicly.
- The dev and proof installs pin exact
  `@inflatable-cookie/poodle-svelte@0.4.16` from the public registry; the
  adapter's published peer is the optional range `>=0.4.16 <0.5` (floor raised
  2026-11-26). Its Rust crates now use tag `v0.4.16` at
  `114c7b5a0bf204b9ea86cccccf7164cb38470586`. Contract 012 carries the full
  checkpoint.
- Superseded: Card 038 artifact set
  `39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`, the
  packable preview this replaced. Kept as record, not as a live pin.
- Every adapter has a framework-neutral client test and a mounted lifecycle
  test.
- DOM-only code is isolated from generated protocol and domain packages.

## Implementation Evidence

Card 039 implements private `@inflatable-cookie/longhorn-poodle-svelte/poodle` against the exact Card 038
artifact. Public Tabs, DockRegion, and SplitView bindings project registered
layout state, serialize expected-revision mutation, reconcile controlled state,
and keep labels, icons, bodies, and static panels in consumer resolvers and
snippets. The root imports no Surface or transfer package. Mounted Nucleus and
Loophole shapes exercise the same adapter.

Card 041 installs three shell shapes from packed artifacts. Public Poodle
theme and presentation setup surrounds distinct consumer frames. Checked
authority loads before guarded reveal; loading, reconnecting, unsupported,
and failed states stay visible. Mounted teardown covers connections and armed
transfer cancellation.

Evidence: the retired logs (Git history),
the retired logs (Git history).

Card 078 applies the same boundary to operations. Per-instance sessions own
listener lifetime and request-keyed pending state. Teardown never requests
host cancellation. Public Poodle progress, status, list, button, and dialog
composition covers Soundcheck scan and Loophole queue fixtures; consumer
detail remains injected.

Evidence: the retired logs (Git history).

## Acceptance

- repeated mount/unmount and destroyed-window tests leave no listener or timer
- late async unlisten is called exactly once
- SSR or build-time import does not access browser globals
- stale optimistic results cannot overwrite a newer authoritative snapshot
- public Poodle adapters cover tabs, regions, and split views without source
  duplication
- cross-window drag uses no Poodle-private selector or payload
- a minimal-shell proof shell and full Loophole shell use different compositions
