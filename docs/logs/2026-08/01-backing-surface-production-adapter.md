# Backing-surface Production Adapter

Date: 2026-08-01
Card: 091
Roadmap: g01.018

## Result

Added `longhorn-native-content-backing-surface` to the production workspace.
It executes coordinator-validated backing-surface plans over one injected
consumer runtime. Longhorn owns generation and host admission, exact partial
receipts, full-host storage evidence, viewport clipping, renderer lifecycle,
physical input admission, stale event rejection, host invalidation, and
reversible detach.

Consumers retain native storage, renderer and pixel production, host-focus
evidence, and typed semantic input. The production graph has no Tauri, AppKit,
GPU, scene, Svelte, Poodle, child-view, or isolated-window dependency.

## Storage, Clip, And Input Policy

Native storage may fill the host while the semantic viewport remains a
smaller physical clip. Moving, resizing, or collapsing the viewport changes
output and interaction authority without moving or detaching storage. Host
resize refreshes full storage independently of the desired clip.

Presentation, non-empty clip and storage, point containment, consumer-supplied
host focus, and `renderer_forwarded` routing all gate input before consumer
semantic dispatch. Focus and visibility observation remain `unknown`; the
adapter does not fabricate native readback from gate state.

Runtime callbacks carry island, host, generation, event sequence, and renderer
frame sequence. Older plans, generations, events, or frame results cannot
replace current evidence. Host destruction invalidates callback authority
before reversible native detach. Failed detach retains the handle for retry.

## Packaged Evidence

The packaged proof uses a real full-host AppKit `NSView` under a transparent
Tauri webview. Raw pointers and deterministic renderer code stay in the proof
runtime. Ten checks pass:

- native storage attaches below the webview
- full-host storage and viewport clip remain distinct
- moved and resized clips change deterministic pixels without moving storage
- zero-area collapse suppresses output and input without detach, then restores
- physical admission precedes consumer-owned semantic payload
- hidden and unfocused states gate presentation or input
- host resize changes storage without rewriting clip
- stale plan and callback evidence leave exact state unchanged
- production source and dependency boundaries remain clean
- host destruction invalidates before exact reversible detach

The available host exposed no distinct native scale, so the live scale
transition remains unmet and unsimulated. Deterministic 1x and 2x conversion
passes in the production test matrix. Windows and Linux are unsupported.

The macOS 26.5.2 arm64 app is 9,172 KiB. Its executable SHA-256 is
`59197902507df2fa4ac34306a739bfca93862ef76bcdc7630278f3ffed5f6a32`.

## Validation

- eight deterministic production adapter tests pass on Rust 1.85
- strict Clippy and formatting pass for the production crate and proof
- the packaged `.app` builds and exits successfully
- the 14-event transcript contains no proof failure
- production graph, inventory, target ledger, report, transcript, docs, and
  focused Effigy checks pass

## Next Task

Execute Card 092. Add per-instance Svelte lifetime and public layout
measurement without a Poodle package edge.
