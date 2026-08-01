# 089 Tauri Child-view Production Adapter

Status: complete
Owner: Tom
Roadmap: g01.018 batch 2
Governing refs: contracts 009, 010, 012, and 017; Card 086
Depends on: Cards 087 and 088
Auto-start next card: no

## Objective

Implement the independently selectable Tauri child-view adapter and prove its
truthful macOS behavior without moving browser or security policy into
Longhorn.

## Scope

- `longhorn-tauri-native-content-child-view`
- isolation of unstable Tauri child-webview APIs
- injected construction and browser/security policy
- attach/reuse, bounds, show/hide, focus request, close, destroy, and teardown
- fresh native observation with valid `unknown` states
- packaged macOS proof and dependency/capability audit
- explicit Windows/Linux `unproved` ledger

## Out Of Scope

- navigation, downloads, popups, permissions, data-store, or URL policy
- isolated-window or backing-surface code
- Svelte/Poodle layout composition
- cross-platform support claims
- Nucleus migration

## Acceptance Criteria

- selected graph omits isolated-window, plugin, GPU, Svelte, and Poodle edges
- remote child content receives no capabilities by default
- hide/show and panel reuse do not recreate content without policy
- bounds use current explicit scale and rounding
- focus and effective visibility remain unknown when not observable
- close, replacement, host destroy, and teardown are exact
- packaged macOS proof records any unavailable live scale transition

## Evidence Required

- packaged behavior report and transcript
- deterministic and live geometry evidence
- security/capability inventory
- target support ledger
- dependency graph and focused Effigy checks

## Stop Conditions

- consumer browser policy must become generic adapter state
- raw webview handles must cross the renderer protocol
- Tauri instability cannot be isolated from the pure package
- current packaged behavior regresses from Card 083 evidence

## Next Task

Execute Card 090. Implement the generic isolated-window/process coordination
layer without importing a plugin ABI.

## Completion

Implemented `longhorn-tauri-native-content-child-view` as a production
workspace crate. It confines Tauri's unstable child-view construction behind
an opaque runtime handle and applies only coordinator-current plans. Runtime
callbacks install before native construction, stay generation-bound, and
cannot revive a closed or invalidated generation.

The adapter owns attach/reuse, physical bounds, show/hide, focus request,
fresh bounds, reversible close, host invalidation, and retryable idempotent
teardown. Browser source, navigation, data-store identity, and Tauri label
mapping are injected. Popup and download persistence default closed. No
remote capability matches the child. Portable focus and visibility remain
`unknown`.

The packaged macOS 26.5.2 arm64 app passes seven behavior checks plus one
explicit unknown observation. It proves creation, readiness, 2x bounds,
hide/show reuse, renderer-unmount independence, browser-policy injection,
close, replacement, teardown, and host destruction. Deterministic 1x/2x
conversion passes. Live scale switching remains unmet because the host
exposed one 2x display. Windows and Linux remain unproved.
