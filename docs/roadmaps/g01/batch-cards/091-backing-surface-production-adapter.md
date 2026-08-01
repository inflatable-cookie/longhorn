# 091 Backing-surface Production Adapter

Status: complete
Owner: Tom
Roadmap: g01.018 batch 2
Governing refs: contracts 003, 009, 012, and 017; Card 086
Depends on: Card 087
Auto-start next card: no

## Objective

Implement generic backing-surface coordination while leaving native storage,
GPU rendering, and semantic input with the consumer.

## Scope

- `longhorn-native-content-backing-surface`
- injected storage, renderer-lifecycle, clipping, and input-gate ports
- full-host storage with viewport render and interaction clip
- visibility, host focus, resize, collapse/restore, destruction, and detach
- deterministic output/outside-clip and stale-result proof
- packaged macOS native-backing proof
- explicit Windows/Linux unsupported ledger

## Out Of Scope

- WGPU device, surface, queue, shaders, scene, camera, picking, or gizmo
- pointer, keyboard, or MIDI semantic payloads
- child-view, isolated-window, Svelte, or Poodle
- Jetstream migration

## Acceptance Criteria

- backing storage may fill the host without widening interaction authority
- output and forwarded input are deterministically clipped to the viewport
- hidden, unfocused, disabled, or empty state gates work explicitly
- stale updates leave current generation unchanged
- destroy and declared reversible detach are exact
- no GPU stack or raw native pointer crosses the adapter API
- packaged macOS proof records any unavailable live scale transition

## Evidence Required

- clip/output/input fixtures and transcript
- resize, collapse, focus, destroy, and detach matrix
- native-boundary and dependency inventory
- target support ledger
- focused Rust, packaged, docs, and Effigy checks

## Stop Conditions

- renderer or semantic input commands must enter the shared protocol
- clip and native storage frame cannot remain distinct
- raw native pointers escape injected platform code
- current packaged behavior regresses from Card 085 evidence

## Next Task

Execute Card 092. Bind the checked client to mounted Svelte viewport and
policy lifetimes without adding a Poodle package edge.
