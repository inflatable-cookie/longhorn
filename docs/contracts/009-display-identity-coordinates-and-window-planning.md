# 009 Display Identity, Coordinates, And Window Planning

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`

## Boundary

Longhorn owns machine-local display identity, typed coordinate conversion,
display correlation, window placement, and deterministic planning. Host
adapters own platform observation and native window mutation. Consumers own
window roles, defaults, and adoption policy.

This contract does not require Surfaces.

## Coordinate Spaces

Geometry always names its space:

| Type | Meaning |
| --- | --- |
| `PhysicalPx` | host or native device pixels |
| `ScreenDip` | global, top-left logical desktop coordinates |
| `ClientCssPx` | local webview content coordinates |
| `ScaleFactor` | explicit conversion evidence |

- No public geometry API accepts an untyped tuple or rectangle.
- Display full bounds and work areas use `ScreenDip`.
- `WindowPlacement` combines an outer origin with an inner content size.
- `LiveWindowMetrics` separately records outer bounds for hit-testing.
- Outer and inner frames are never substituted for each other.
- Host adapters own conversion and documented rounding at their boundary.
- Client coordinates reach screen space only through current window metrics.

## Display Identity

- `DisplayId` is an opaque, machine-local Longhorn identity.
- A new known display receives an id once and retains it until explicitly
  forgotten.
- Platform ids, hardware UUIDs, names, position, size, scale, and built-in
  flags are observations and correlation evidence. None is universally
  canonical.
- Canonical ids are not synchronized between machines.
- Known displays may remain absent without losing labels or placement memory.

Correlation runs strongest evidence first:

1. strong platform or hardware key
2. remembered adapter key
3. exact geometry and scale
4. unique weak fingerprint

Every result records its confidence and evidence. Ambiguous weak matches do
not bind automatically or overwrite remembered evidence. A consumer may ask
the user to resolve ambiguity.

An arrangement signature sorts available canonical ids with full bounds,
work areas, scale, and main-display status. Adapter enumeration order is not
significant.

## Window Planning

Planning is pure and deterministic over known displays, current observations,
saved placement, live windows, and consumer policy.

- A configured display is tried before ordered fallbacks.
- A required primary window falls back through largest useful intersection,
  main display, then the first deterministic available display.
- Required windows are clamped to the target work area and minimum size.
- Temporary fallback does not rewrite the configured home display.
- A settled user move may adopt a new attached display when consumer policy
  permits it.
- Maximized state stores and restores its normal placement separately.
- No available display produces an explicit unavailable plan, not fabricated
  geometry.

The live-versus-desired plan emits explicit operations such as create,
retag, move/resize, maximize, show, focus, and close. Apply generations let
the host suppress feedback from its own mutations.

## Host Boundary

- Tauri monitor and window values are converted at the adapter edge.
- The host captures user move and resize only after settling and flushes on
  close or shutdown.
- Screen-point hit-testing uses live outer bounds.
- Window labels are transport identifiers, not domain identity.
- Host failures return typed errors and leave the desired model inspectable.

## Acceptance

- type checks prevent physical, screen-logical, and client-local mixing
- property tests cover round-trip conversion and documented rounding
- Loophole display-correlation fixtures preserve confidence and ambiguity
- Nucleus restore fixtures select saved, intersecting, main, then deterministic
  fallback displays
- window placement tests distinguish outer origin, inner size, and outer bounds
- temporary fallback never silently changes a saved home display
- pure planning imports no Tauri, Svelte, Poodle, or Surface package

