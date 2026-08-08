# Notification Clients, Svelte, Poodle, And Semantic Actions

Date: 2026-07-31
Card: 080
Status: complete

## Changed

- Added the Rust-authoritative exact-v1 notification protocol, checked
  TypeScript generation, and golden mutation/event fixture.
- Added framework-neutral direct, serialized, and Tauri clients with strict
  compatibility, correlation, listener-first reconciliation, and paging.
- Added `longhorn-tauri-notifications` with injected caller authority, narrow
  read/manage capabilities, and app-wide invalidation hints.
- Added a per-instance Svelte session and controlled public-Poodle retained
  panel, `ToastHost`, and `ToastStack` adapters.
- Added explicit seen/dismiss commands, renderer-local toast removal, and an
  injected semantic-action executor called at click time.

## Decisions

- The first snapshot restores retained truth but never creates transient
  toasts. Only newly observed unseen records selected by consumer policy do.
- Poodle owns timer lifetime. Its store dismissal removes only the local toast
  projection; ledger seen/dismiss transitions require explicit commands.
- Changed events are non-durable invalidation hints. Any newer revision, gap,
  or authority epoch reloads a bounded authoritative page.
- Action references remain bounded data. Consumers resolve and authorize them
  afresh on every invocation; no generic execute-by-string host command exists.
- The framework-neutral root depends only on `@inflatable-cookie/longhorn-core`. Tauri, Svelte,
  Poodle, operation, and command edges remain removable.

## Evidence

- Rust and generated TypeScript consume the same checked golden fixture.
- Direct, JSON-serialized, and mocked Tauri traces converge exactly.
- Two renderer controllers and two mounted Svelte sessions keep local
  selection isolated while shared seen/dismiss commits converge.
- Gap, duplicate/stale event, epoch change, response correlation, late
  teardown, remount, and SSR fixtures pass.
- Mounted public `ToastHost` expiry removes the toast while the retained panel
  and authority record remain. Remount reloads the record without replay.
- A semantic action made inadmissible after render is rejected by the injected
  executor at invocation, proving fresh admission.
- Focused Rust tests, Clippy, TypeScript, Svelte checks, Bun tests, Vitest, dry
  package assembly, binding drift, formatting, and Northstar path checks pass.

## Next

Execute Card 081. Pack isolated operation/notification consumers, compare
native and renderer traces, publish composition guidance, and close g01.012.
