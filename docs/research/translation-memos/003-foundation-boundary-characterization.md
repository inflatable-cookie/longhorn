# Foundation Boundary Characterization

Status: promoted  
Owner: Tom  
Updated: 2026-07-28

## Question

Which display, window, IPC, drag, lifecycle, and package behaviors are proven
across the audited Tauri apps, and which proposed abstractions are new?

## Repositories

- `loophole`
- `nucleus`
- `soundcheck`
- `jetstream`
- `acowtancy/bovine-accelerator-desktop`

All five are Svelte 5 and Tauri 2 applications. Rust editions and package
manager choices differ.

## Display And Window Evidence

Loophole has the strongest display inventory:

- client ids are process-specific observations
- correlation runs through exact geometry/scale, geometry-only, then size-only
- matches record confidence
- known displays retain labels, overrides, geometry, scale, and per-client ids
- macOS can supply a stronger Core Graphics display UUID
- window planning is pure and guarantees a fallback for a required window

Nucleus persists native physical geometry and uses a synthetic
name/position/size display key. Restore selects saved display, largest
intersection, primary, then first. This key is useful evidence but changes
with rearrangement and is not durable identity.

Tauri distinguishes outer position, inner size, and outer size. Loophole's
current capture combines outer position with inner size for placement, while
screen-point window hit-testing uses outer bounds.

Promoted to contracts 009 and 012:

- machine-local canonical display ids with confidence-bearing correlation
- explicit physical, screen-logical, and client-local coordinate spaces
- outer-origin plus inner-size placement distinct from live outer bounds
- pure window planning and narrow Tauri apply adapters

## Drag Evidence

Loophole panel drag is currently same-webview HTML5 drag through Poodle. Its
renderer projects a local drag state and commits a host panel mutation.

Loophole's proven cross-window operation moves whole Surfaces. The renderer
reports a screen point after leaving the source window. Rust resolves a
managed target window, moves the Surface, or may create a window on empty
display space.

Cross-window panel transfer is therefore a new Longhorn capability. Contract
011 limits it to id-only sessions, leased screen-space target zones,
authoritative re-resolution, and transactional commit. Empty-display window
creation is explicit consumer policy, not inherited panel behavior.

## IPC And Lifecycle Evidence

Jetstream centralizes Tauri invocation, installs a state listener before its
ready signal, emits a full state snapshot, shares one handler assembly between
the app and mock tests, and returns asynchronous unlisten functions. Its
snapshots do not carry revisions.

Nucleus already uses `ts-rs` for generated DTOs. Soundcheck has a large
handwritten invoke surface and inconsistent event teardown. Together they
support:

- Rust-authoritative generated TypeScript contracts
- central raw transport wrappers
- testable handler assembly
- listener-before-snapshot handshake
- epochs and revisions for deduplication, gaps, and restart
- explicit async teardown behavior

These conclusions are promoted to contracts 010 and 013.

## Package Evidence

The apps span Rust 2021 and 2024, Svelte 5, Tauri 2, and Bun/npm workflows.
The common library therefore needs package-manager-neutral npm artifacts,
narrow peer-based adapters, and Rust compatibility that accepts both consumer
editions.

The promoted topology uses edition 2024 with MSRV 1.85, coordinated package
versions, optional capability packages, and no g01 umbrella package.

## Remaining Research

- non-macOS strong display evidence and ambiguity UX
- packaged multi-window drag behavior across platforms and display scales
- exact registry names before first publication

## 2026-07-28 Display Revalidation

Read-only donor inspection reconfirmed the promoted display/window boundary:

- Loophole `echo-display-inventory`, `display_correlation`,
  `display_registry`, and `echo-windowing` retain canonical/observed display
  separation, scale-in-thousandths, confidence tiers, labels, remembered
  client ids, configured fallbacks, and per-display geometry.
- Nucleus `nucleus-workspaces::{geometry,displays,planning}` and desktop
  `window_geometry` retain known-versus-available records, saved/fallback
  planning, largest-intersection selection, primary/first fallback, negative
  desktop origins, and work-area clamping.
- Soundcheck `app_settings` retains the minimal single-window case: outer
  origin plus inner size, primary/first fallback, explicit `320x240` minimum,
  work-area clamp, debounce, and close flush.

The pure lane therefore splits into typed geometry, display correlation,
placement resolution, and desired/live diffing. Tauri observation/mutation,
event settling, debounce, persistence, and ambiguity UI remain outside
`g01.003`. No donor repository was modified.
