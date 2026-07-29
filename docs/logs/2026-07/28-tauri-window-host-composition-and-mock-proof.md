# Tauri Window Host Composition And Mock Proof

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 021
- added one runtime-generic assembly for mock and native Tauri runtimes
- bound predeclared stable identity, protected-primary policy, listeners,
  apply, reveal, lifecycle events, shutdown, and teardown
- reused app-managed initialization without duplicate listeners or slots
- installed dynamic-window lifecycle handling before later native operations
- restored registry ownership after successful and failed apply planning
- added typed initialization, apply, reveal, shutdown, and teardown evidence
- made teardown flush once and return a typed repeat no-op
- added protected-main and `workspace-*` capability examples
- documented simple and dynamic consumer integration

## Composition

Nucleus uses one protected predeclared window and `NoWindowFactory`. Loophole
uses the same host plus an injected dynamic factory. Soundcheck uses the
single-window shape with its two-second close bound. No fixture or package
imports layout, Surfaces, config, TypeScript, Svelte, Poodle, or consumer code.

The host removes its registry from shared state during apply. Factory, native
mutation, readback, and reveal therefore run without the registry lock.
Planning failure restores the same registry before returning. Dynamic create
inserts the managed slot, installs lifecycle handling, then permits dependent
geometry and visibility work. Failure rolls the slot back and reports stable
id, created handle, generation, and failed call where available.

Tauri window listeners have no returned unlisten handle. Teardown marks the
lifecycle host inactive, clears installed targets, and leaves weak callbacks
as no-ops. The app retains the torn-down host, so later assembly returns
`Reused` without creating a second listener set.

## Capability Policy

Both example capability files parse through Tauri's `Capability` schema. They
grant only `core:window:allow-start-dragging`. One matches `main`; the dynamic
form matches `main` and `workspace-*`. Rust-hosted mutation, probing, capture,
and persistence add no renderer permission.

## Evidence

- eight Card 021 mock composition tests pass
- Nucleus hidden restore, capture, reveal, shutdown, repeated initialization,
  and idempotent teardown pass
- Loophole protected main and dynamic workspace creation pass
- Soundcheck two-second close and sink-failure receipt pass
- duplicate initialization, planning, factory, native apply, probe/readback,
  unknown event, sink request, and flush failure paths remain typed
- exact generation, stable id, handle, completed calls, and failed call are
  asserted for native failure
- capability files parse and retain exactly one permission
- all 37 `longhorn-tauri-windowing` tests pass
- warnings-denied package Clippy passes
- integration guide links pass
- full Effigy QA passes

Rust 1.85 fails before Longhorn compilation because the locked Tauri graph
contains ICU packages requiring 1.86 and Darling, plist, serde_with, and time
packages requiring 1.88. Card 022 owns dependency-floor reconciliation.

## Posture

`strict-paused`

Card 021 is complete. No card is ready automatically. Card 022 remains at the
packaged-proof checkpoint.

## Next

The native macOS environment and Rust 1.85 dependency-floor plan are now
recorded. Explicitly start Card 022.
