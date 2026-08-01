# Tauri Child-view Production Adapter

Date: 2026-07-31
Card: 089
Roadmap: g01.018

## Result

Added `longhorn-tauri-native-content-child-view` to the production workspace.
It executes current `longhorn-native-content` plans through coordinator-
validated receipts and confines Tauri's unstable `WebviewBuilder` and
`Window::add_child` APIs to one runtime module.

The public adapter retains an opaque handle, not `tauri::Webview`. It supports
attach/reuse, physical bounds, show/hide, focus request, fresh bounds, close,
generation replacement, host invalidation, and retryable idempotent teardown.
Callbacks install before native construction and are rejected after their
generation closes or becomes stale.

## Policy Boundary

Consumers inject logical-to-Tauri labels, source URL, navigation admission,
and optional macOS data-store identity. The built-in runtime denies popup
creation and download persistence. The example capability matches only a
trusted local controller and grants no commands; no capability matches remote
child content.

Portable child focus and effective visibility readback remain `unknown`.
Tauri has no portable child-webview input-disable operation. A `disabled`
input request therefore fails exactly instead of claiming an unenforced gate.
Consumers may hide the child or later inject a stronger proved target runtime.

## Packaged Evidence

The macOS 26.5.2 arm64 package is 9,164 KiB. Its executable SHA-256 is
`104ecd8efe99d84530bacea72b5976293ff5c719a8a9bd732be7cd29c6f3cd10`.

Seven packaged checks pass:

- controlled remote child creation and readiness
- fresh 2x physical bounds
- injected browser policy and closed capability posture
- hide/show and renderer-unmount reuse without recreation
- explicit scale and rounding for moved bounds
- deterministic 1x/2x conversion
- close, replacement, teardown, host destroy, and late-event rejection

Focus and visibility are one explicit `observed_unknown` result. Live scale
switching is unmet: the host exposed one 2x display. Windows and Linux remain
unproved.

The first packaged run correctly rejected a direct `Attached -> Absent`
observation after close. The final proof records adapter `DetachStarted`
evidence as `Detaching` before admitting fresh `Absent`, preserving contract
017 instead of weakening it.

## Validation

- eight deterministic adapter tests pass on Rust 1.85
- production adapter and proof checks pass
- strict Clippy and formatting pass
- packaged `.app` build and clean runtime transcript pass
- selected graph omits isolated-window, backing-surface, plugin, GPU, Svelte,
  and Poodle edges
- committed inventory, target ledger, report, and transcript pass focused
  Effigy verification

## Next Task

Execute Card 090. Implement generic isolated-window and process-boundary
coordination without importing Signal, plugin ABI, unsafe unload, or outer
placement authority.
