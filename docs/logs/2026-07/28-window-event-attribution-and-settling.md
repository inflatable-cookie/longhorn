# Window Event Attribution And Settling

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 019
- added a pure per-window lifecycle coordinator to `longhorn-windowing`
- made monotonic time and every interval caller-owned
- attributed matching native geometry and close events to exact apply evidence
- preserved bounded user precedence across later programmatic marks
- added generation-coalesced settling and persistence debounce
- emitted capture, bounded flush, user-close, and terminal-forget directives
- reset failed capture generations so a later explicit flush can retry
- made Card 020 the sole ready lane

## Attribution

The host registers each exact `WindowOperation` and apply generation before
native mutation. Move/resize evidence matches exact desired placement.
Create, maximize, and unmaximize retain only the transition categories they can
honestly predict. Close consumes explicit close evidence. An attribution
deadline cannot prove origin without matching evidence.

A mismatching geometry event starts or extends the caller-configured user
precedence interval. Matching evidence registered later cannot suppress events
during that interval. Stale generation and timestamp evidence cannot replace
current state.

## Settling And Flush

User geometry schedules complete live capture after a caller-supplied quiet
period. A newer event replaces the pending capture generation. Completion
schedules a separate persistence-debounce deadline. Blur captures immediately.
Explicit flush and user close force pending capture first when needed.

Every flush directive carries the caller's timeout. User close reports
consumer policy without changing desired state. Destroy always emits bounded
flush plus forget and releases coordinator state, including reordered terminal
delivery.

## Evidence

- Loophole's 3-second attribution, 5-second user precedence, and
  300-millisecond settle policy are supplied as fixture values
- Nucleus and Soundcheck opt out with zero attribution
- exact move/resize/scale and accumulated maximize transitions are covered
- a user drag outranks a later matching apply generation
- stale timestamps, generations, deadlines, duplicate completion, and checked
  deadline overflow remain typed
- blur, programmatic close, expired close evidence, user close, explicit flush,
  and destroy are covered
- seventeen focused lifecycle tests pass
- `longhorn-windowing` checks successfully on Rust 1.85
- warnings-denied Clippy, rustdoc, full Effigy QA, and current-toolchain
  workspace tests pass
- the lifecycle implementation adds no god-file finding; Doctor reports only
  the repository's 35 existing warning-level findings and no errors

The complete workspace cannot currently resolve under Rust 1.85 because
existing Tauri-side transitive versions require Rust 1.86-1.88. The pure
Card 019 package has no such dependency and passes its declared floor. This
workspace dependency-floor mismatch remains visible for distribution
closeout.

## Boundary

No Tauri type, listener, clock, sleep, thread, channel, IO, persistence,
configuration, layout, Surface, Poodle, product policy, or donor write entered
the coordinator.

## Posture

`strict-ready`

Card 019 is complete. Card 020 is ready against the explicit directive and
generation seams.

## Next

Review and explicitly start Card 020. Do not bind native Tauri events
automatically.
