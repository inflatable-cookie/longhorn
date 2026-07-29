# 016 Live Window Diff Planning

Status: complete
Owner: Tom
Roadmap: g01.003 batch 3
Governing refs: contracts 001, 003, 009, and 012; research memo 003
Auto-start next card: no

## Objective

Complete `g01.003` with a pure desired-versus-live window diff, explicit apply
generations, and inspectable operations for the later Tauri host.

## Scope

- desired window snapshot built from Card 015 placements
- live host-window snapshot with stable window identity and transport handle
- live outer bounds separate from desired outer-origin/inner-size placement
- create, retag, move/resize, maximize, unmaximize, show, hide, focus, and close
- deterministic operation order and idempotent empty diff
- explicit protected primary host slot
- apply generation carried by every programmatic operation
- feedback-suppression evidence without event handling
- typed host capability and unavailable-operation diagnostics
- no-Surface and hosted-Surface-shaped fixtures

## Public Behavior

Diffing is pure. It receives desired windows, live windows, host capabilities,
focus policy, and one caller-supplied apply generation. It emits ordered
operations and diagnostics without calling a host.

Stable `WindowId` is domain identity. A Tauri label or other host handle is
transport identity. Retagging is explicit; labels never become window ids.
Move/resize compares the desired outer screen origin and inner content size
against corresponding live metrics. Live outer bounds remain available for
screen hit-testing but cannot substitute for placement.

The protected primary host slot is reused or retagged according to explicit
input policy and is never closed by inference. Reapplying a live snapshot that
already matches the desired snapshot produces no operations.

## Out Of Scope

- executing operations
- Tauri error mapping, event listeners, debounce, settling, or shutdown flush
- persistence and configuration updates
- layout containers, panels, Surfaces, or cross-window drag
- focus-stealing product policy beyond explicit inputs
- TypeScript, Svelte, Poodle, or donor-repository writes

## Steps

1. Define desired, live, capability, generation, operation, diagnostic, and
   diff-receipt types.
2. Match desired and live windows by stable identity before transport labels.
3. Plan protected-primary reuse and explicit retagging.
4. Plan create, move/resize, maximize state, visibility, focus, and close.
5. Order operations deterministically so creation and retag precede geometry,
   geometry precedes visibility/focus, and close runs last.
6. Attach the supplied apply generation to every programmatic operation.
7. Add idempotence, capability, feedback-evidence, and consumer-shaped fixtures.
8. Run complete validation and close `g01.003`. Stop at the `g01.004` host
   planning gate.

## Acceptance Criteria

- matching uses stable `WindowId`, never transport label identity
- protected primary slot is not closed by inference
- create and retag precede geometry operations
- geometry precedes show and focus
- close operations are last and deterministic by `WindowId`
- move/resize compares outer origin and inner size, not outer bounds
- live outer bounds remain available for hit-testing
- maximize and unmaximize are explicit
- unsupported host capabilities return diagnostics without fabricated success
- every programmatic operation carries the caller generation
- an already matching live snapshot emits an empty idempotent diff
- permuting desired or live input does not change ordered output
- no-Surface and hosted-Surface-shaped fixtures produce equivalent window plans
- package graph has no Tauri, config, layout, Surface, Svelte, or Poodle edge

## Evidence Required

- create, retag, move/resize, maximize, show, focus, and close unit fixtures
- protected-primary replacement and reuse fixtures
- outer-origin/inner-size versus outer-bounds regression test
- operation-order permutation tests
- idempotent second-diff test
- unsupported-capability diagnostics
- apply-generation propagation and stale-feedback fixture
- Nucleus no-Surface and Loophole hosted-Surface-shaped fixtures
- Rust 1.85 workspace check
- `effigy doctor`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`

## Stop Conditions

- diffing must call Tauri or mutate a native window
- a host label must become stable window identity
- outer bounds must stand in for inner content size
- operation order depends on input enumeration
- a protected host slot can be closed without explicit policy
- event settling, debounce, persistence, layout, or Surface state enters scope
- Tauri, config, Svelte, Poodle, Surface, or donor types are required

## Next Task

`g01.003` and Card 017 are complete. `g01.004` remains compiled through Card
022; Card 018 is the sole ready lane.
