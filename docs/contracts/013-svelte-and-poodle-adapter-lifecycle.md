# 013 Svelte And Poodle Adapter Lifecycle

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`

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

## Reactive State

- Epoch and revision determine snapshot freshness.
- Renderer projections never become durable fallback truth.
- Optimistic mutations are keyed by request id, reconciled to the host result,
  and rolled back or resynced on failure.
- Errors, loading, reconnecting, and unsupported capability states are
  explicit.

## Poodle Boundary

- Adapters use public controlled props, snippets, and events.
- Poodle owns tabs, dock regions, split views, menus, dialogs, and visual
  semantics.
- Longhorn owns ids, eligibility, command dispatch, and authoritative state.
- Consumer panel bodies and product presentation remain slots or snippets.
- No adapter imports Poodle internals or copies Poodle source.

The titlebar-drag helper rejects interactive descendants and modified pointer
gestures before asking the host to start native dragging. Failure goes through
an injected reporter.

## Compatibility

- Svelte, Poodle, and Tauri compatibility ranges are declared peers.
- Every adapter has a framework-neutral client test and a mounted lifecycle
  test.
- DOM-only code is isolated from generated protocol and domain packages.

## Acceptance

- repeated mount/unmount and destroyed-window tests leave no listener or timer
- late async unlisten is called exactly once
- SSR or build-time import does not access browser globals
- stale optimistic results cannot overwrite a newer authoritative snapshot
- public Poodle adapters cover tabs, regions, and split views without source
  duplication
- a minimal Bovine shell and full Loophole shell use different compositions

