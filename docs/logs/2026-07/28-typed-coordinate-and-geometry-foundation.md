# Typed Coordinate And Geometry Foundation

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added opaque machine-local `DisplayId` and product-neutral `WindowId`
- added distinct physical-pixel, screen-DIP, and finite client-CSS geometry
- added typed points, sizes, vectors, and rectangles without tuple APIs
- added positive scale evidence in integer thousandths
- added explicit floor, ceil, and nearest conversion with typed overflow
- added checked area, containment, intersection, translation, fitting, and
  minimum-visibility operations
- separated desired outer-origin/inner-size placement from live outer bounds
- added deterministic serde, compile-fail fixtures, property tests, and donor
  cases

## Geometry Policy

Integral physical and screen geometry uses signed `i32` origins and `u32`
extents. Rectangle edges widen before arithmetic. Translation returns typed
overflow. Full fitting and minimum visibility are separate operations.
Consumer minimums remain inputs: Soundcheck's `320x240` rule is a fixture, not
a core constant.

Client CSS values are finite `f64` values. Negative zero normalizes to zero.
Client sizes reject negative extents. No client-to-screen conversion exists in
core; that conversion requires current window metrics at a later boundary.

Scale evidence is any positive `u32` thousandths. Integer conversion always
names rounding. Nearest physical-to-DIP-to-physical error is bounded by
`ceil(scale_thousandths / 2000)` physical pixels; identity scale is exact.
This replaces the draft one-pixel claim, which is not mathematically true
above 3000 thousandths.

## Evidence

- `proptest 1.11.0` declares Rust 1.85 and is dev-only with `std`
- compile-fail fixtures reject physical/screen, client/screen, and
  placement/live substitution
- property tests cover conversion quantization, widened intersection,
  containment, translation overflow, fitting, visibility, and serde
- Nucleus negative-origin and oversized-window fixtures pass
- Soundcheck input-supplied minimum fixture passes
- Loophole desired-placement versus live-outer-bounds fixture passes
- `longhorn-core` normal dependencies remain `serde` only
- Rust 1.85 workspace check passed
- formatting and warnings-denied Clippy passed
- workspace tests passed

## Validation Note

One immediate repeated workspace run inside the first Effigy QA attempt timed
out three existing configuration helper-process tests. The preceding full run
passed. All three exact retries passed. Final Effigy QA passed.

## Boundary

No display inventory, correlation, fallback, live diffing, Tauri,
configuration, TypeScript, Svelte, Poodle, Surface, product type, or donor
write entered `longhorn-core`.

## Posture

`strict-ready`

Card 013 is complete. Card 014 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start Card 014 display inventory and correlation.
