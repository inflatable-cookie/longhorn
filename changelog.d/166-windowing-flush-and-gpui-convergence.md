Explicit window-placement flushes are now bounded. `ConfigWindowPlacementSink`
starts each publication on a worker and returns its acknowledgement channel, so
`flush_window_placements` reports `TimedOut` when a publication outlasts the
request's timeout. A publication that times out may still complete afterwards,
and its placements stay staged until one succeeds.

Explicit flushes also refuse new publications once four are still in flight,
returning `RequestFailed` instead of starting another worker. A caller that keeps
timing out can no longer accumulate blocked workers.

GPUI apply now refuses to report desired state reached while the fresh readback
still has a remaining operation or diagnostic, or cannot compute its diff.
`capture_from_gpui_facts` refuses a fullscreen window instead of persisting it
as a windowed placement. Programmatic resizes and maximizes that an apply makes
are attributed to the apply and no longer captured as user placements.

A GPUI retag publishes the old identity's staged placements first, and is
refused if that publication does not succeed. It then moves the window's
pending capture and flush work and its apply evidence to the new identity, so
the window's own resizes stay attributed to the apply after the retag.

Breaking:

- `execute_gpui_window_apply` takes a fifth argument, `observer: &mut impl
  GpuiApplyObserver`. Pass the `GpuiWindowLifecycleHost` that will see the
  window events; it implements the trait. A harness with no lifecycle host must
  pass its own observer and say so.
- `GpuiApplyObserver` is new. Its `register_apply` and `retag` methods are what
  the apply calls before each native mutation and each identity change. `retag`
  returns `GpuiRetagError`, which is new: `Refused` leaves the registry
  unchanged, and `Rescheduling` means the window moved but a pending deadline
  could not be re-issued, so the registry moves too and the attempt reports it.
- `GpuiWindowCall` gains `LifecycleBookkeeping`, and `GpuiApplyFailureKind`
  gains `Lifecycle`. Exhaustive matches need an arm for each.
- `ConfigWindowPlacementSink`'s `WindowPlacementSink` implementation requires
  the consumer domain to be `'static`. Migration: an owned domain type already
  satisfies this; a domain that borrows must own its data instead.
- `ConfigWindowPlacementSink::request_flush` no longer writes on the caller's
  thread. A caller that drops the ticket gets no synchronous write. Migration:
  wait for the flush through `flush_window_placements`, or on the ticket.
- `ConfigWindowPlacementSink::domain` is no longer a `const fn`.
- `ConfigWindowPlacementSink` refuses a publication with `RequestFailed` while
  four are still in flight. A caller that keeps timing out must wait for an
  earlier flush to drain before its next request is accepted.
