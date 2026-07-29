# Surface Identity, Topology, And Resolution

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 028
- added bounded `SurfaceId`, `SurfaceRequestId`, and checked
  `SurfaceRevision`
- added the pure optional `longhorn-surfaces` crate
- added explicit finite document and resolution limits
- bound every Surface to one distinct opaque layout container
- represented fallback priority and per-window tab order without window
  geometry
- validated identity, reference, uniqueness, count, order, label, and active
  membership invariants
- canonicalized structural records without changing host priority or tab order
- accepted only current admitted Surface ids and available participating
  windows
- returned deterministic resolved windows and typed unresolved Surfaces
- kept lifecycle, persistence, native apply, transfer, TypeScript, and UI out
- left donor repositories unchanged
- made Card 029 ready

## Public Shape

One Surface record contains:

- `SurfaceId`
- one distinct `LayoutContainerId`
- optional bounded label
- ordered `SurfaceHostPreference` entries

Each host preference contains a participating `WindowId` and its complete
zero-based tab order for that window. The entry sequence is fallback priority.
Each participating window stores only its preferred active Surface.

The document contains no layout body, panel state, display, geometry, native
handle, product attachment, or presence expression.

## Resolution Table

| Input | Result |
| --- | --- |
| admitted; preferred window available | preferred window |
| admitted; preferred missing; declared fallback available | first available fallback |
| admitted; no declared candidate available | unresolved `no_available_window` |
| not admitted | unresolved `not_admitted` |
| saved active resolves in window | saved active |
| saved active does not resolve in window | first resolved member |
| empty available window | no active Surface |

Resolution validates and normalizes a private document clone. It never changes
the source document, rewrites preferences, repairs topology, or creates a
window.

## Invariant Evidence

The contract suite covers:

- empty, malformed, and excessive Surface identity
- zero and hard-limit rejection
- Surface, window, host, order, and layout-binding duplicates
- missing and unknown hosts
- incomplete per-window order
- invalid active membership
- excessive Surface, window, preference, and label counts
- strict serde and product-payload rejection
- idempotent normalization
- document and external-input permutation stability
- absent presence and missing-window outcomes
- malformed admitted and available sets
- three-Surface/two-window Loophole composition

Existing Nucleus conformance stays in `longhorn-layout`. Neither
`longhorn-layout` nor its fixture graph imports `longhorn-surfaces`.

## Package Report

| Package | Ordinary dependencies |
| --- | --- |
| `longhorn-surfaces` | `longhorn-core`, serde |
| `longhorn-layout` | `longhorn-core`, serde |

`longhorn-surfaces` contains no config, filesystem, window host, layout body,
Tauri, TypeScript, Svelte, Poodle, or product dependency. Window and layout
bindings use opaque ids from `longhorn-core`.

## Validation

- 12 focused Surface contract tests pass
- Surface and core tests pass
- warnings-denied Surface Clippy passes
- Rust 1.85 workspace all-target check passes
- full Effigy QA passes
- dependency inspection confirms the narrow package graph
- `git diff --check` passes

## Remaining Limits

- no expected-revision lifecycle commands
- no persistence adapter
- no external proof that a bound layout container currently exists
- no native window composition
- no generated TypeScript
- no transfer session or drag behavior
- no consumer migration

These remain assigned to Cards 029-035.

## Posture

`strict-ready`

## Next

Start Card 029: authoritative Surface lifecycle and registered persistence.
