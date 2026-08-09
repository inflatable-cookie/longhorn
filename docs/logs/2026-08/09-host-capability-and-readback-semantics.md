# Host Capability And Readback Semantics

Date: 2026-08-09
Card: 163 follow-up (batch 3)
Roadmap: g02.012

## Result

Three of the four bends Card 163 recorded but could not fix are fixed. They
were blocked by the greenfield source freeze, not by anything technical; the
freeze lifted and they took one batch.

`effigy qa` green end to end.

## Shape

**`HostCapability::MoveResize` is now `Move` and `Resize`.** Separate
operations, separate capabilities, diffed per axis by the pure planner.

**`WindowDiffInput::desired_windows` is public**, and
`execute_gpui_window_apply` no longer takes the duplicate parameter that
carried desired state past the freeze.

**`DeferredSettlement`** names the operations a host cannot observe settling
in the same turn. `WindowDiffReceipt::without_deferred` drops them from a
convergence diff and keeps every diagnostic.

## Decisions

**A capability names one operation.** A compound cannot describe a host that
has half of it. GPUI has `Window::resize` and no position setter, so under
`MoveResize` it withheld both and a GPUI window could not be resized from a
plan at all — the adapter placed it once at creation and never again. Now it
declares `Resize`, withholds `Move`, and reaches half the placement for real
while naming the half it cannot. Tauri declares both and is unaffected except
that a window which only drifted sideways is no longer resized back to a size
it already had.

**Diagnostics survive deferral; operations do not.** `without_deferred` drops
planned operations and keeps diagnostics, because they are different claims.
An operation that has not settled yet will settle. An unsupported operation is
a fact about the host and will not become supported by waiting.

**The regression test's fake reproduces the real failure, not a convenient
one.** `with_lagging_maximize` accepts the call and keeps reporting the old
state — which is exactly what macOS did when the smoke binary drove a real
window. The default fake returns what it was just told, which is the fake
anyone would write and the reason the in-memory suite could not have found
this.

## Consequential Changes

Two behavioural shifts fell out of the split. Both are covered by tests rather
than left to be discovered:

- A partial placement failure now reports honestly. Previously a failed
  `SetInnerSize` produced one `Failed` attempt that listed the *successful*
  `SetOuterPosition` among its completed calls. Now the move succeeds as its
  own attempt and only the resize fails.
- Operations sort by kind, so placement batches per axis across windows —
  every origin, then every size — rather than per window across axes.
  Independent windows do not care.

`InstalledWindow` retains placement per axis for the same reason: a pure move
no longer carries a size, and composing a normal placement from a
half-applied one would invent the other half.

## Evidence

- `longhorn-windowing` 37 tests, `longhorn-tauri-windowing` 49,
  `longhorn-gpui-windowing` 30
- `prototypes/gpui-windowing` still compiles against gpui 0.2.2 and its smoke
  binary still opens a real window at the exact planned origin
- `effigy qa` green, including all twelve artifact proofs

## What Is Still Open

Moving `CountingProbe`, `transfer_session_probe` and `operation_probe` out of
`longhorn-tauri-update`. None reference Tauri. That one was never blocked by
the freeze — `longhorn-tauri-update` is Card 162's live surface, and it waits
for 162 to land rather than colliding with it.

Contract 020 remains incomplete on the coverage grounds it states for itself:
no backend has proved multi-window placement, cross-window transfer, or
lifecycle teardown under load.
