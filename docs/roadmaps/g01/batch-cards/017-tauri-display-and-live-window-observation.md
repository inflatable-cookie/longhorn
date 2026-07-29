# 017 Tauri Display And Live Window Observation

Status: complete
Owner: Tom
Roadmap: g01.004 batch 1
Governing refs: contracts 001, 009, and 012; research memos 003 and 008
Auto-start next card: no

## Objective

Add `longhorn-tauri-windowing` with checked Tauri 2 display and managed-window
observation over the pure Card 013-016 types.

## Scope

- workspace package depending on core, display, windowing, and Tauri only
- Tauri 2.10.3 API floor proven by Loophole and Soundcheck
- generic `tauri::Runtime`; no hidden Wry runtime requirement
- checked Tauri scale-factor conversion to positive integer thousandths
- raw physical monitor and window facts preserved at the edge
- injected whole-desktop coordinate mapper
- built-in uniform-scale mapper
- explicit nearest rounding at the host edge
- process-local observation ids and optional injected evidence
- `DisplayBuiltinStatus::{Unknown, BuiltIn, External}` in `longhorn-display`
- exact main-display matching and machine-label observations
- managed live-window outer position, outer size, inner size, maximize,
  visibility, and focus facts
- complete snapshot or typed failure

## Public Behavior

The adapter converts Tauri facts; it does not correlate or allocate canonical
display identity. Monitor name and observation order remain evidence only.
Callers may inject strong or remembered adapter evidence.

The built-in mapper converts a complete uniform-scale desktop with explicit
nearest rounding. Mixed-scale arrangements require one injected mapper over
the complete raw snapshot. Per-monitor origin division is rejected because
Tauri does not guarantee that it forms one coherent global logical plane.

Managed windows are supplied explicitly. A label remains
`HostWindowHandle`; stable identity comes from caller bookkeeping. If any
managed window cannot provide required scale, geometry, visibility, focus, or
maximize state, the entire managed snapshot fails. The planner never receives
a false absence.

## Out Of Scope

- display correlation or id allocation
- native mutation or dynamic creation
- protected-primary retagging
- event listeners, settling, debounce, persistence, reveal, or flush
- layout, Surfaces, TypeScript, Svelte, Poodle, or donor writes

## Steps

1. Replace the boolean built-in fact with strict snake-case
   `DisplayBuiltinStatus` and update signatures, serde, and fixtures without a
   compatibility shim.
2. Add the narrow workspace crate and compatible Tauri dependency.
3. Define typed observation and probe errors with operation and handle context.
4. Convert validated Tauri scale to `ScaleFactor`.
5. Define `DesktopCoordinateMapper` over complete raw display/window facts and
   add the uniform-scale implementation.
6. Reject mixed-scale mapping when no provider is injected.
7. Match the primary monitor exactly or fail ambiguous attribution.
8. Project Tauri observations into `ObservedDisplay` without canonical ids.
9. Probe explicitly managed webview windows into `LiveWindow`.
10. Reject incomplete managed snapshots.
11. Add raw-fact, mock-runtime, permutation, and donor-shaped fixtures.

## Acceptance Criteria

- invalid, zero, non-finite, and overflowing scale or geometry fail typed
- negative physical origins survive checked DIP conversion
- uniform-scale conversion uses exact whole-plane scale evidence
- mixed-scale conversion without a mapper is explicitly unavailable
- per-monitor origin division is absent
- full bounds and work areas remain distinct
- unknown built-in status cannot become external by default
- ambiguous primary matching fails typed
- outer bounds and inner content size remain distinct
- monitor name and enumeration index never become `DisplayId`
- observation ids are process-local only
- stable window identity never comes from a Tauri label
- one failed managed probe prevents a partial live snapshot
- unmanaged windows cannot enter or block the managed snapshot
- package graph contains no config, layout, Surface, TS, Svelte, Poodle, or
  consumer dependency

## Evidence Required

- Loophole multi-monitor logical conversion fixture
- Nucleus negative-origin physical monitor fixture
- Soundcheck single-monitor fixture
- fractional scale and invalid-scale fixtures
- uniform-scale negative-origin and mixed-scale unavailable fixtures
- injected mixed-scale mapper fixture
- full/work-area and inner/outer-frame regression fixtures
- incomplete managed-window probe fixture
- input permutation and serde evidence where applicable
- Rust 1.85 workspace check
- `effigy doctor`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`

## Stop Conditions

- Tauri facts must become canonical display or window identity
- a failed managed probe must be silently omitted
- conversion requires ambient rounding or unchecked casts
- mixed-scale origins must be divided independently and treated as global DIPs
- the adapter must own product window definitions
- config, layout, Surface, UI, or donor types enter the package

## Next Task

Card 018 is ready against the implemented complete-snapshot and managed-window
seams. Review it before starting native mutation.
