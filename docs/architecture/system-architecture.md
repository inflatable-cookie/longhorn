# System Architecture

Status: promoted first pass  
Owner: Tom  
Updated: 2026-07-27  
Vision: `../vision/001-shared-tauri-systems.md`

## Boundary

Longhorn owns reusable desktop mechanisms and their cross-language contracts.
Consumer apps own product topology, panel catalogues, resource attachments,
workflow state, and presentation composition.

Poodle owns visual primitives including tabs, dock regions, split views,
menus, dialogs, and presentation tokens. Longhorn supplies state machines,
policies, Tauri adapters, and Poodle bindings. It does not copy Poodle
components.

## Composable Hosting Model

Regions belong to an opaque layout container.

```text
display inventory -> window plan
                         |
                         +-> window as layout container -> regions -> panels
                         |
                         +-> hosted surfaces -> surface as layout container
                                                -> regions -> panels
```

This removes Surface from the mandatory core hierarchy:

- Nucleus binds a `WindowId` to a layout-container id.
- Loophole enables the Surface package and binds a `SurfaceId` to a
  layout-container id.
- Panel and region logic does not need to know which hosting choice was made.

## Target Layers

### Foundation model

- opaque stable ids
- logical and physical bounds
- intersection, clamping, and scale conversion
- versioned local-state envelopes
- deterministic normalization

No Tauri, Svelte, Poodle, or product dependency.

### Display inventory

- host monitor probe input
- canonical display records
- current availability
- arrangement signature
- known-display correlation and recovery
- machine and user labels

Tauri monitor APIs are an adapter, not the domain model.

### Window planning and host

- configured target and ordered display fallbacks
- geometry memory per display
- deterministic window plan
- pure live-versus-desired apply plan
- restore, clamp, debounce, and close flush
- user-versus-programmatic move attribution
- dynamic Tauri webview-window adapter

This layer does not depend on Surfaces.

### Layout core

- consumer-defined region ids and families
- panel ids, definitions, allowed regions, and instance policy
- panel ordering and active-panel state
- region sizing, collapse, visibility, and normalization
- mutation commands and deterministic resolution
- layout-container id as the only parent requirement

Panel bodies and panel resource attachments remain consumer-owned.

### Optional Surface hosting

- stable Surface identity and labels
- window hosting preferences and fallback
- ordered hosted Surfaces per window
- active Surface per window
- presence gates
- create, duplicate, close, move, and reorder lifecycle
- focused-panel and regional habitat policy

This is a separate package/module. Apps that omit it carry no Surface state.

### Local state and persistence

- schema version and migrations
- atomic file replacement
- process-local serialization
- merge-safe partial updates
- debounced scheduling and explicit flush
- corruption and missing-file policy

Storage format and location are injected. Product state does not enter the
generic store.

### TypeScript and Svelte

- typed snapshots and commands derived from or checked against Rust authority
- framework-neutral placement and drag helpers
- Svelte stores/actions for subscriptions and command dispatch
- Poodle `Tabs`, `DockRegion`, and `SplitView` adapters
- safe custom-titlebar drag helper
- hidden compatible-region reveal during panel drag

Renderer state stays transient. Durable placement changes return through the
host-authoritative snapshot.

### Tauri bridge

- command/event contract registration
- listener lifetime and current-snapshot handshake
- testable command handler assembly
- primary-window coordinator identity
- capability examples for dynamic windows

The bridge must not become a product command bus.

## Drag And Drop

- Same-webview tab and region movement uses Poodle's HTML5 payload contract.
- Cross-webview/window movement carries ids and pointer/screen coordinates,
  never serialized model state.
- The Rust host resolves the current authoritative snapshot and target
  window/display.
- Pointer-math editing gestures remain consumer or specialist-library work.

## Authority

- machine adapter observes displays and native windows
- Longhorn resolves generic placement and lifecycle rules
- consumer host owns persistence location and product policy
- consumer renderer owns presentation and transient interaction
- Poodle owns component behavior and visual semantics

## Package Shape

Exact names remain provisional. Package seams, not a monolith, are required:

- Rust foundation/display/window/layout packages
- optional Rust Surface hosting package
- Tauri adapter package
- TypeScript protocol/core package
- Svelte/Poodle integration package

## Validation Strategy

- pure Rust fixtures for display/window/layout resolution
- serialization fixtures shared across Rust and TypeScript
- Tauri mock-runtime command tests
- Vitest coverage for drag, reorder, subscription, and adapter logic
- consumer conformance fixtures from Loophole and Nucleus
- packaged-app proofs for native window and cross-window behavior
