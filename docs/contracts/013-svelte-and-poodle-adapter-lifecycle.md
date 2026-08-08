# 013 Svelte And Poodle Adapter Lifecycle

Status: active compiled boundary
Owner: Tom
Updated: 2026-07-29
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`,
`../research/translation-memos/011-client-svelte-poodle-and-shell-boundary.md`,
`../research/translation-memos/014-command-input-and-palette-boundary.md`

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
  after teardown.
- Window destruction releases listeners, timers, drag leases, and pending
  optimistic projections.
- Transport and scheduler dependencies are injectable for tests.
- A shared lifecycle primitive may own registration, pump, and teardown. Each
  domain still supplies validation and freshness comparison.
- A client is added only for a real host command/event contract. Checked
  generated layout data does not imply an invented layout service.

## Reactive State

- Epoch and revision determine snapshot freshness.
- Renderer projections never become durable fallback truth.
- Optimistic mutations are keyed by request id, reconciled to the host result,
  and rolled back or resynced on failure.
- Errors, loading, reconnecting, and unsupported capability states are
  explicit.
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
  conflict records, and injected dispatch.
- A command selection calls a consumer executor. No Svelte or Poodle adapter
  invents a generic Tauri execution route.
- Same-window drag remains Poodle-owned. Cross-window binding requires public
  drag start, end, eligibility, and external-drop extension points.
- Poodle `g12.016` supplies those public points through typed asynchronous
  prepare, synchronous start, terminal end/cancel, target eligibility, and
  accepted-drop callbacks. Pending preparation emits no external payload.
- Private MIME values, generated DOM ids, CSS classes, and event-target
  reverse engineering are not public integration seams.

The titlebar-drag helper rejects interactive descendants and modified pointer
gestures before asking the host to start native dragging. Failure goes through
an injected reporter.

## Shell Composition

- Theme and presentation bootstrap uses public Poodle token and provider APIs.
- Readiness reveal is explicit and follows authoritative load.
- Missing capability and host errors remain visible states.
- Guidance composes these pieces; Longhorn does not ship one mandatory app
  frame.

## Compatibility

- Svelte, Poodle, and Tauri compatibility ranges are declared peers.
- Private Poodle adapter proof pins one exact source commit and packable preview
  artifact until the later release lane proves a public range.
- The current pin is Card 038 artifact set
  `39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.
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

Evidence: `../logs/2026-07/29-poodle-layout-bindings.md`,
`../logs/2026-07/29-three-shape-app-shell-proof-and-closeout.md`.

Card 078 applies the same boundary to operations. Per-instance sessions own
listener lifetime and request-keyed pending state. Teardown never requests
host cancellation. Public Poodle progress, status, list, button, and dialog
composition covers Soundcheck scan and Loophole queue fixtures; consumer
detail remains injected.

Evidence: `../logs/2026-07/31-operation-svelte-session-and-poodle-projection.md`.

## Acceptance

- repeated mount/unmount and destroyed-window tests leave no listener or timer
- late async unlisten is called exactly once
- SSR or build-time import does not access browser globals
- stale optimistic results cannot overwrite a newer authoritative snapshot
- public Poodle adapters cover tabs, regions, and split views without source
  duplication
- cross-window drag uses no Poodle-private selector or payload
- a minimal Bovine shell and full Loophole shell use different compositions
