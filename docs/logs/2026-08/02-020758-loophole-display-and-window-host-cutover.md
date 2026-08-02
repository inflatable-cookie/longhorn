# Loophole Display And Window Host Cutover

Date: 2026-08-02
Time: 02:07:58 Europe/London
Card: 106
Roadmap: g01.015

## Result

Aura's workspace windows now use one Longhorn Tauri host:

- protected hidden `main`, retagged logically without native recreation
- dynamic `workspace-*` windows from injected Loophole title, URL, chrome, and
  minimum-size policy
- canonical display reconciliation and arrangement signatures
- placement resolution, exact apply generations, fresh readback, guarded
  reveal, settled capture, close, aggregate flush, and bounded teardown

Plugin editor and other native windows remain outside the managed registry.
The host diagnostic exposes active state, generation, initial convergence,
managed count, domain and import paths, arrangement, display/window counts,
and the last complete receipt.

## Persistence And Import

The registered `loophole.window-placement` machine-state domain owns known
displays, allocator state, attached ids, arrangement signature, enabled state,
home/fallback display policy, per-display normal geometry, and maximized state.
It cannot overwrite workspace layout or Surface topology.

First use imports the retained Echo display registry and all geometry from the
workspace window projection. Publication is target-first. A missing receipt
can resume from an existing target. The receipt binds the target, legacy
`windowing.json`, and workspace projection digests and records that sources
remain retained. Corrupt legacy display state stops startup before publication.

## Retained Product Policy

Loophole still owns:

- logical required/optional roles and window titles
- display adoption only when the old home remains attached
- temporary fallback without rewriting an absent home
- last-window and last-Surface close behavior
- focused-Surface fullscreen
- platform canonical-id, label, built-in, and logical-geometry evidence

Longhorn owns the generic inventory, placement, native apply, capture, reveal,
and lifecycle mechanisms. `echo-configuration` remains import-only here.
`echo-windowing` remains only as the shell topology donor for Cards 107-108.

## Longhorn Correction

Protected-slot retag previously changed apply-registry identity but left the
lifecycle listener and readiness gate under the old logical id. Retag now
moves lifecycle identity by stable transport handle before registry commit.
The event listener resolves current identity from that handle. A composition
regression proves readiness works only under the new id.

## Duplicate And Capability Audit

Removed from Aura:

- `services/window_apply.rs`
- `WindowApplyCoordinator`
- three-second programmatic suppression
- five-second user-move disambiguation
- 300 ms donor geometry debounce
- donor diff planner and dynamic-window builder
- direct Echo display correlation and machine-registry mutation

The only remaining workspace `WebviewWindowBuilder` is the injected Longhorn
factory. Tauri config grants one `workspace-windows` capability to `main` and
`workspace-*`. Plugin GUI labels do not match it. No permission expansion was
needed.

## Evidence

Consumer tests prove:

- all per-display donor geometry imports
- target/receipt publication retains both sources byte-for-byte
- settled moves adopt an attached landed display
- temporary fallback capture retains the absent home
- serialized two-display state survives restart
- home loss selects remembered fallback geometry and return restores the home

Shared host tests prove exact generation, partial failure, suppression,
capture, reveal, close, aggregate flush, teardown, dynamic creation, protected
primary behavior, and retag lifecycle identity.

## Validation

- Longhorn Tauri window-host suite: 45 passed
- Aura native suite: 190 passed
- Aura renderer suite: 1,031 passed
- Aura Svelte check: zero diagnostics
- Loophole Effigy validation: passed
- Rust formatting and diff checks: passed

## Next Task

Execute Card 107. Move Loophole's eight-region layout model and mutation path
to one registered Longhorn layout authority.
