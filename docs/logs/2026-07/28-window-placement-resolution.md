# Window Placement Resolution

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added pure `longhorn-windowing` over `longhorn-core` and `longhorn-display`
- added stable window config with required-primary and optional roles
- added configured home and ordered configured fallback selection
- added largest-useful-intersection, main, and canonical fallback recovery
- added per-display normal placement with explicit caller default
- added fitted work-area placement with caller minimum size
- added minimum-visible extent as useful-intersection policy
- kept maximized state separate from fitted normal geometry
- added unavailable and disabled outcomes without fabricated geometry
- added pure settled-placement memory and home-adoption proposals

## Resolution Policy

Configured available home wins, followed by the first available configured
fallback. Optional windows stop there. Required-primary windows continue
through largest useful intersection, current main display, then the first
canonical available `DisplayId`.

Intersection usefulness comes from caller-supplied minimum visible extent.
Equal areas retain the lowest canonical `DisplayId`; adapter enumeration order
cannot decide. Selected geometry uses target-specific memory when present,
then saved home/fallback memory, then the caller default. Card 013 fitting
applies the caller minimum size and keeps the result inside the selected work
area.

Temporary fallback carries the unchanged configured home and an explicit
reason. Settled user placement returns a per-display normal-memory update.
Configured-home adoption appears only when the caller selects
`AdoptAttachedDisplay`. Neither path mutates or persists configuration.

## Donor Evidence

- Nucleus saved, intersection, main, deterministic-first, negative-origin,
  rearranged, tie, and unavailable cases pass
- Loophole configured target, ordered fallback, target-specific memory,
  maximized separation, and hosted-Surface-shaped cases pass
- Soundcheck's `320x240` minimum and work-area fitting pass as policy inputs
- no-Surface and hosted-Surface-shaped consumers produce the same result

## Donor Delta

Nucleus treats any positive intersection as useful. Longhorn makes that
threshold explicit through minimum visible extent; a `1x1` policy preserves
the donor behavior. Main and first-display fallbacks are canonical-id
deterministic rather than host enumeration dependent.

Loophole's pure donor plan can derive a host-ordered fallback when every
configured window misses. Longhorn permits that recovery only for the
explicit required-primary role and uses intersection, main, then canonical id.
Optional windows remain unavailable instead of being fabricated.

## Evidence

- 16 focused `longhorn-windowing` conformance and serde tests pass
- temporary fallback leaves configured home unchanged
- maximized outputs and proposals retain separate normal geometry
- malformed empty work areas fail with typed geometry evidence
- normal dependencies are `longhorn-core`, `longhorn-display`, and `serde`
- Rust 1.85 workspace check passed
- formatting, warnings-denied Clippy, workspace tests, and Effigy QA passed
- Effigy Doctor reports zero errors and no new size warning

## Boundary

No Tauri mutation, native inventory, event settling, feedback suppression,
debounce, persistence, configuration store, layout, Surface, TypeScript,
Svelte, Poodle, product type, or donor write entered the package.

## Posture

`strict-ready`

Card 015 and the later Card 016 are complete. `g01.003` is closed.

## Next

Review and explicitly start Card 017.
