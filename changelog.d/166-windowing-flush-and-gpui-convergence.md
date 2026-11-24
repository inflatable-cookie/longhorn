Explicit window-placement flushes are now bounded. `ConfigWindowPlacementSink`
starts each publication on a worker and returns its acknowledgement channel, so
`flush_window_placements` reports `TimedOut` when a publication outlasts the
request's timeout. A publication that times out may still complete afterwards,
and its placements stay staged until one succeeds.

GPUI apply now refuses to report desired state reached while the fresh readback
still has a remaining operation or diagnostic, or cannot compute its diff.
`capture_from_gpui_facts` refuses a fullscreen window instead of persisting it
as a windowed placement. Programmatic resizes and maximizes that an apply makes
are attributed to the apply and no longer captured as user placements. The
GPUI lifecycle host now receives apply evidence and identity changes, so a
retag keeps its installed window and waits while that window has pending work.

Breaking:

- `execute_gpui_window_apply` takes a fifth argument, `observer: &mut impl
  GpuiApplyObserver`. Pass the `GpuiWindowLifecycleHost` that will see the
  window events; it implements the trait. A harness with no lifecycle host must
  pass its own observer and say so.
- `GpuiApplyObserver` is new. Its `register_apply` and `retag` methods are what
  the apply calls before each native mutation and each identity change.
- `GpuiWindowCall` gains `LifecycleBookkeeping`, and `GpuiApplyFailureKind`
  gains `Lifecycle`. Exhaustive matches need an arm for each.
- `ConfigWindowPlacementSink`'s `WindowPlacementSink` implementation requires
  the consumer domain to be `'static`. Migration: an owned domain type already
  satisfies this; a domain that borrows must own its data instead.
- `ConfigWindowPlacementSink::request_flush` no longer writes on the caller's
  thread. A caller that drops the ticket gets no synchronous write. Migration:
  wait for the flush through `flush_window_placements`, or on the ticket.
- `ConfigWindowPlacementSink::domain` is no longer a `const fn`.
