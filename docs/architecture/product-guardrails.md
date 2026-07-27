# Product Guardrails

Status: active  
Owner: Tom  
Updated: 2026-07-27  
Vision: `../vision/001-shared-tauri-systems.md`

## Reuse Bar

- require two-app evidence or a clear stable mechanism-level case
- extract behavior, not filenames
- keep consumer policy injectable
- reject generic APIs that still require donor-domain types

## Composition

- Surface support is optional
- simple single-window apps remain simple
- no mandatory runtime service for local desktop state
- package dependencies follow capability direction

## UI Boundary

- Poodle owns visual primitives
- Longhorn owns desktop state and integration
- Svelte packages should remain usable without copying app shells

## Delivery

- characterize donor behavior before moving it
- add shared conformance fixtures
- migrate a real consumer in the same lane
- remove superseded donor copies after cutover
- do not claim extraction complete on library scaffolding alone

## Safety

- restore windows within current usable display bounds
- flush material local state on close
- never trust cross-window drag payload state
- preserve app-specific permissions and authorization boundaries
