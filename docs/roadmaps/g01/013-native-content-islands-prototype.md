# g01.013 Native Content Islands Prototype

Status: prototype  
Owner: Tom  
Updated: 2026-07-27

## Outcome

Find the smallest shared host seam across child webviews, isolated native
inspection windows, and embedded render surfaces.

## Batches

### 1. Characterization

- Nucleus browser-panel child webviews
- Soundcheck isolated plugin inspection
- Jetstream WGPU view beneath a transparent webview
- lifecycle, bounds, scale, visibility, focus, and input matrix

### 2. Prototype seam

- island id and host capability description
- geometry/visibility apply plan
- lifecycle and event/input adapters
- no renderer-owned durable native truth

### 3. Decision

- run packaged proofs on supported platforms
- promote one adapter family, split by mechanism, or reject sharing
- document Poodle/Svelte composition limits

## Acceptance

- prototypes use current Tauri/native APIs in packaged apps
- scale, occlusion, destruction, and focus behavior is observable
- a generic API exists only if it removes real duplication

