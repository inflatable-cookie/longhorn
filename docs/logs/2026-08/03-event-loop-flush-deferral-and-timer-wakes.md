# Event-loop Flush Deferral And Timer Wakes

Date: 2026-08-03
Card: 139
Roadmap: g02.002

## Result

The Tauri event thread no longer blocks on bounded flushes, one shared timer
thread replaces the parked-thread-per-wake scheduler, superseded wakes cancel
instead of firing to be ignored, and every scheduler-delivered wake outcome —
success or failure — now reaches the `WindowLifecycleReporter`.

## Shape

- `handle_tauri_event` now requires `Arc<Self>` and defers directive flushes
  to the blocking pool. The receipt carries a new
  `TauriWindowLifecycleAction::FlushDeferred` action; the terminal `Flushed`
  outcome arrives as a later reporter receipt for the same window and event
  kind.
- `handle_lifecycle_event` (wake and consumer path) keeps exact inline
  flush-at-directive-position semantics via an internal `FlushDisposition`;
  shutdown keeps its bounded aggregate.
- `TauriAsyncWindowLifecycleScheduler` now owns one named timer thread with a
  monotonic heap; a newer wake for the same window and event kind supersedes
  an undelivered older one. The idle worker exits when its host is gone.
- The host's `WindowLifecycleWakeHandler` impl reports both Ok receipts and
  typed failures; scheduler-path wakes were previously invisible on both
  sides.
- `WindowLifecycleEventKind` gained `Hash`/`Ord` derives for supersession
  keys.

## Behavior Note

On event-thread close/destroy, the consumer user-close callback now runs
before the deferred flush completes (previously the flush blocked first).
Staged data remains in the sink; the bounded `shutdown_flush` aggregate is
the durability backstop at exit. Recorded as intentional.

## Exact Evidence

- destroyed-with-held-sink returns in <1s against a 2s flush timeout with
  `FlushDeferred`, and the `TimedOut` outcome arrives as a reporter receipt
- superseded same-window same-kind wake is never delivered; only the latest
  fires
- unknown-window wake delivery reports `Err(UnknownWindow)` through the
  reporter
- tauri-windowing 48 tests, windowing 40 tests, Clippy, and full workspace
  `--all-targets` check pass
