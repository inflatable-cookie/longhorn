# 013 Typed Coordinate And Geometry Foundation

Status: complete
Owner: Tom
Roadmap: g01.003 batch 1
Governing refs: contracts 001, 009, and 012; research memo 003
Auto-start next card: no

## Objective

Add framework-independent coordinate, geometry, scale, display-id, and
window-id primitives to `longhorn-core` without Tauri, window policy, or
consumer types.

## Scope

- opaque machine-local `DisplayId` and product-neutral `WindowId`
- typed physical-pixel, screen-DIP, and client-CSS coordinate values
- typed points, sizes, and rectangles with no tuple-based public geometry
- positive fixed-point scale evidence in thousandths
- explicit rounding mode for physical/logical conversion
- checked conversion and rectangle edge arithmetic
- intersection, containment, area, translation, and clamping
- explicit minimum size and minimum visible extent inputs
- outer-origin and inner-content-size distinction in type shape
- deterministic serialization and property tests

## Public Behavior

Physical, screen-logical, and client-local values are different Rust types.
No public operation silently mixes them. Durable scale evidence uses positive
integer thousandths; `1000` means `1.0`. Zero scale, non-finite client values,
invalid ids, and unrepresentable conversion results fail typed.

Physical/logical conversion always names its rounding mode. There is no
ambient or platform default in `longhorn-core`. Rectangle math uses widened,
checked intermediates so negative desktop origins and large dimensions cannot
overflow silently.

Clamping receives minimum window size and minimum visible extent as policy
inputs. It does not hard-code Soundcheck, Nucleus, Loophole, operating-system,
or Poodle behavior.

## Out Of Scope

- known or observed display records
- correlation and arrangement signatures
- display fallback or window planning
- maximized-state policy
- live window diffing
- Tauri monitor/window conversion
- persistence or configuration domains
- TypeScript, Svelte, Poodle, Surface, or donor-repository writes

## Steps

1. Characterize a Rust 1.85-compatible property-test dependency.
2. Add opaque display and window ids with strict serde round trips.
3. Add typed coordinate scalars, points, sizes, rectangles, and scale factor.
4. Add explicit checked physical/DIP conversion and rounding modes.
5. Add checked intersection, containment, area, translation, and clamp helpers.
6. Model outer origin and inner content size without substituting outer bounds.
7. Port negative-origin, oversized, minimum-size, and minimum-visibility donor
   fixtures.
8. Run the complete card validation. Stop before display inventory.

## Acceptance Criteria

- physical, screen-DIP, and client-CSS APIs cannot be mixed by type
- scale factor rejects zero and serializes as stable thousandths
- conversion names rounding and rejects overflow or invalid client values
- identity conversion at `1000` is exact
- nearest physical-to-DIP-to-physical round trips stay within the documented
  `ceil(scale_thousandths / 2000)` physical-pixel quantization bound
- intersection is commutative and never exceeds either input area
- contained rectangles remain unchanged by clamping
- oversized windows shrink within explicit minimum and work-area bounds
- negative desktop origins remain valid
- outer-origin plus inner-size placement cannot be passed as live outer bounds
- public serialization is deterministic
- `longhorn-core` retains no Tauri, Svelte, Poodle, Surface, or product dependency

## Evidence Required

- compile-fail or type-level fixtures for coordinate-space mixing
- serde fixtures for ids, scale, points, sizes, and rectangles
- property tests for conversion, overflow, intersection, containment, and clamp
- Nucleus negative-origin and oversized-window cases
- Soundcheck `320x240` minimum-policy fixture supplied as input, not a constant
- Loophole outer-origin/inner-size versus outer-bounds fixture
- Rust 1.85 workspace check
- `effigy doctor`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`

## Stop Conditions

- a public geometry API needs an untyped tuple or rectangle
- one coordinate space can be substituted for another without conversion
- rounding or overflow would be implicit
- a consumer minimum size or visibility rule must become a core constant
- display correlation or window policy leaks into `longhorn-core`
- Tauri, Svelte, Poodle, Surface, or a donor type becomes a dependency

## Next Task

Cards 014-017 and `g01.003` are complete. Card 018 is the sole ready
`g01.004` lane.
