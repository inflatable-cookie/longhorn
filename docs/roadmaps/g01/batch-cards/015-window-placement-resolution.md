# 015 Window Placement Resolution

Status: complete
Owner: Tom
Roadmap: g01.003 batch 3
Governing refs: contracts 001, 003, 009, and 012; research memo 003
Auto-start next card: no

## Objective

Add pure `longhorn-windowing` placement resolution over known displays,
current inventory, saved per-display geometry, and explicit consumer policy.

## Scope

- product-neutral window role and stable `WindowId`
- enabled, required-primary, optional, and unavailable window outcomes
- configured home display and ordered fallback displays
- normal placement remembered independently per canonical display
- outer screen-DIP origin plus inner content size
- maximized state with separate normal placement
- explicit minimum size and minimum visible extent policy
- configured, ordered fallback, intersection, main, and deterministic fallback
- temporary fallback without home-display mutation
- pure settled user-placement reconciliation with explicit adoption policy
- Loophole, Nucleus, and Soundcheck fixtures

## Public Behavior

Placement resolution is deterministic over immutable inputs. It first tries the
configured home display, then ordered configured fallbacks. A required primary
window then tries the display with the largest useful intersection, the current
main display, and the first canonical available display. Equal intersections
break by `DisplayId`, never adapter enumeration.

The resolved normal placement is clamped to the target work area through Card
013 geometry policy. Maximized state never overwrites normal placement.
Temporary fallback records why another display was used and does not rewrite
the configured home.

A settled user placement returns a proposed memory update. Home-display
adoption occurs only when explicit consumer policy permits it. The pure package
does not persist, debounce, observe Tauri events, or mutate a native window.

## Out Of Scope

- live host-window inventory and desired/live diff operations
- Tauri creation, mutation, event capture, settling, or feedback suppression
- configuration storage and flush
- Surface hosting or layout-container state
- cross-window drag
- TypeScript, Svelte, Poodle, or donor-repository writes

## Steps

1. Add `longhorn-windowing` depending only on `longhorn-core` and
   `longhorn-display`.
2. Define placement config, per-display normal memory, policy, reason, resolved,
   unavailable, and proposed-update types.
3. Implement configured home and ordered fallback selection.
4. Implement largest-useful-intersection, main, and canonical deterministic
   fallback for a required primary.
5. Clamp through explicit minimum-size and minimum-visibility policy.
6. Keep normal placement separate from maximized state.
7. Implement pure settled-placement memory and optional home-adoption proposal.
8. Port donor fixtures and run complete validation. Stop before live diffing.

## Acceptance Criteria

- configured available home wins
- first available configured fallback wins when home is missing
- required primary then selects largest useful intersection
- equal intersection ties resolve by canonical `DisplayId`
- main display precedes deterministic first display when no useful intersection
- no display returns explicit unavailable state
- optional windows may remain unavailable without fabricated placement
- output records whether placement is home, configured fallback, intersection,
  main, or deterministic emergency fallback
- minimum size and visible extent come only from consumer policy
- target placement stays within the selected work area
- maximized output retains distinct normal placement
- temporary fallback does not change configured home
- settled move changes home only when adoption policy permits
- package graph has no Tauri, config, layout, Surface, Svelte, or Poodle edge

## Evidence Required

- Nucleus saved, intersection, main, deterministic-first, and no-display cases
- Soundcheck single-window minimum-size and work-area clamp case
- Loophole configured target, ordered fallbacks, and per-display geometry case
- negative-origin and rearranged-display cases
- equal-intersection permutation test
- temporary fallback and explicit-adoption state tests
- maximized normal-placement preservation test
- no-Surface and Surface-shaped consumer fixtures yielding the same placement
- Rust 1.85 workspace check
- `effigy doctor`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`

## Stop Conditions

- fallback selection depends on host enumeration order
- temporary fallback must rewrite configured home
- maximized bounds must replace normal placement
- product minimums or display preferences become library constants
- native mutation, debounce, persistence, layout, or Surface state enters scope
- Tauri, config, Svelte, Poodle, Surface, or donor types are required

## Next Task

Cards 016-017 and `g01.003` are complete. Card 018 is the sole ready
`g01.004` lane.
