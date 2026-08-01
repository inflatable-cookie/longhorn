# Native-content Production Kernel

Date: 2026-07-31
Card: 087
Roadmap: g01.018

## Result

Added `longhorn-native-content`, the first production artifact from the
promoted native-content graph. It owns pure identity, desired and observed
state, mechanism-specific viewport planning, proposals, host invalidation,
and exact apply receipts.

Shared `NativeContentIslandId`, `NativeContentKindId`, failure/reason ids, and
`NativeContentRevision` live in `longhorn-core`. The new crate has only
`longhorn-core` and `serde` as normal dependencies.

## Production Decisions

- Attach generations and plan steps are nonzero, including during decode.
- One generation binds one logical host. A host change advances generation.
- Mechanism capabilities declare their active input route. `disabled` remains
  the common gate; another unsupported route fails before mutation.
- Child view uses complete physical bounds. Isolated window uses content size
  without outer placement. Backing surface uses viewport clip while retaining
  separate storage bounds.
- Detach is not reissued after observation reaches `detaching`.
- Host destruction atomically records absent observation and invalidates the
  generation before late events can enter. Repeating the event is idempotent.
- Apply receipts require a current island, desired revision, observed
  revision, and non-invalidated generation. Sparse reports retain failure,
  not-attempted, and dependency-skipped causality.
- Content-size proposals are non-mutating and revision/generation bound.

These tighten the Card 082 prototype without making its API compatibility
authority.

## Evidence

Seventeen production contract tests cover all three shapes, deterministic
geometry, invalid construction and decode, stale revision/generation,
capability rejection, lifecycle, host destruction, detach, partial receipts,
stale completion, proposal admission, payload exclusion, and dependency
isolation.

Focused Rust tests and Clippy pass. Full workspace and Northstar validation is
recorded by the Card 087 closeout gate.

## Roadmap Result

Card 087 is complete. Card 088 is ready for checked Rust-to-TypeScript
generation and the framework-neutral client. Native mechanisms remain behind
Cards 089-091; donor writes remain blocked through Card 093.
