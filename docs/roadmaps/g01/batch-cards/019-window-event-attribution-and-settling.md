# 019 Window Event Attribution And Settling

Status: complete
Owner: Tom
Roadmap: g01.004 batch 2
Governing refs: contracts 001 and 009; research memo 008
Auto-start next card: no

## Objective

Add a pure coordinator that distinguishes programmatic apply effects from
settled user placement and emits lifecycle directives without Tauri or IO.

## Scope

- per-window apply generation and expected-state evidence
- programmatic close evidence
- move, resize, scale, blur, close, and destroy inputs
- explicit monotonic clock input
- caller-supplied attribution, settle, debounce, and flush bounds
- user-activity precedence over later programmatic marks
- generation-based coalescing
- directives for ignore, schedule capture, capture now, flush, user close, and
  forget
- deterministic late and stale event handling

## Public Behavior

An apply generation is registered before native mutation. Events attributable
to its expected transition do not become user placement. A user-attributed
sequence retains precedence for its explicit hold interval.

Elapsed time is policy and expiry evidence, not origin by itself. Loophole's
3-second suppression, 5-second user window, and 300-millisecond debounce remain
fixtures. Longhorn has no donor timing defaults.

Close classification consumes explicit programmatic-close evidence. User close
produces a directive for consumer policy; it does not disable a window.

## Out Of Scope

- Tauri event types or listeners
- threads, sleeps, channels, or filesystem writes
- geometry probing or display correlation
- product close policy
- reveal, configuration, layout, Surfaces, or UI

## Steps

1. Define monotonic timestamps, policy bounds, event inputs, and directives.
2. Register apply generation, expected operation/state, and close evidence.
3. Classify apply-driven versus user-driven geometry events.
4. Preserve user activity across interleaved programmatic marks.
5. Coalesce settle and debounce generations deterministically.
6. Model blur, close, destroy, stale deadlines, and explicit flush.
7. Add fake-clock and input-permutation fixtures.

## Acceptance Criteria

- no wall clock or sleep enters pure logic
- all timing bounds are caller inputs
- apply evidence is installed before affected events
- stale generation cannot suppress current user state
- user activity precedence is explicit and bounded
- programmatic close cannot become user close
- user close cannot mutate desired state inside the coordinator
- duplicate and reordered stale events converge deterministically
- destroy clears per-window state

## Evidence Required

- Loophole 3s/5s/300ms fixture supplied as policy
- Nucleus and Soundcheck no-suppression restore fixtures
- programmatic move/resize/maximize sequence fixture
- user drag interleaved with a new apply mark
- stale generation, late deadline, blur, close, and destroy fixtures
- fake-clock property or exhaustive transition tests
- Rust 1.85 and full Effigy QA

## Stop Conditions

- one donor interval must become a library constant
- elapsed time alone must prove programmatic origin
- product disable/delete policy enters the coordinator
- Tauri, threads, persistence, layout, or Surface types are required

## Outcome

`longhorn-windowing` now exposes a pure per-window lifecycle coordinator.
Callers provide monotonic timestamps and all timing policy. Exact
`WindowOperation` evidence classifies matching move, resize, scale, and close
events. User activity has an explicit bounded precedence window.

Settling and persistence debounce use independent capture generations.
Directives schedule or request complete capture, schedule or request bounded
flush, report user close, and forget terminal state. Stale timestamps,
generations, deadlines, and duplicate completion return inspectable ignore
reasons. Destroy is terminal even when callback delivery is reordered.

Seventeen focused fixtures cover Loophole's supplied 3s/5s/300ms policy,
Nucleus/Soundcheck zero-attribution policy, move/resize/maximize evidence,
interleaved user precedence, blur, close, destroy, explicit flush, checked
deadline overflow, failed-capture reset and retry, serde, and exhaustive
settle/debounce transitions. The
package compiles on Rust 1.85 without Tauri, clocks, threads, IO, persistence,
layout, Surface, or product policy.

## Next Task

Card 020 is ready against the explicit event, capture-generation, flush-bound,
user-close, and forget directives. Review it before binding real Tauri events.
