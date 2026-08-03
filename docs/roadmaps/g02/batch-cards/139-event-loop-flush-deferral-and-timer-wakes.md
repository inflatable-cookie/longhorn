# 139 Event-loop Flush Deferral And Timer Wakes

Status: complete
Owner: Tom
Roadmap: g02.002 batch 1
Governing refs: contracts 001, 010, and 017; research memo 018
Depends on: none
Auto-start next card: no
Completed: 2026-08-03

## Objective

Keep the Tauri event loop free of synchronous flush waits and replace the
parked-thread wake scheduler with cancelable timer wakes whose delivery
failures leave evidence.

## Scope

- `crates/longhorn-tauri-windowing/src/lifecycle/host/directives.rs` flush
  execution path
- `crates/longhorn-tauri-windowing/src/lifecycle/services.rs` scheduler and
  wake delivery
- coordinator-emitted `Flush` handling for `Destroyed` and user-close

## Steps

1. Defer event-path `Flush` directives off `handle_tauri_event` (mirror the
   shutdown `pending_flushes` batching) so `ticket.wait()` never runs on the
   event thread.
2. Replace `spawn_blocking` + `thread::sleep` with an async timer task; make
   superseded generations cancel instead of waking to be ignored.
3. Route `handle_scheduled_wake` failures through `WindowLifecycleReporter`
   instead of `let _ =`.
4. Add proofs: destroy-with-pending completes without event-thread blocking;
   drag-storm scheduling occupies no blocking-pool threads; a failed wake is
   reported.

## Acceptance Criteria

- no lifecycle path blocks the event thread up to `flush_timeout`
- pending flushes still complete or report failure on destroy and close
- wakes cancel on supersession; delivery failures reach the reporter
- windowing suites, mock host proofs, and workspace QA pass

## Evidence Required

- event-thread non-blocking proof receipts
- cancellation and reporter-delivery test receipts
- QA receipts

## Stop Conditions

- flush completion semantics require a pure-coordinator protocol change
- Tauri's async runtime cannot express a cancelable timer without a new
  dependency

## Evidence

- deferred event-path flush with `FlushDeferred` receipt action and later
  reporter outcome; inline semantics retained for wake/consumer paths
- one shared timer thread, supersession cancellation, reporter-visible wake
  outcomes
- regression tests for non-blocking destroy, supersession, and failure
  reporting; 48 crate tests, Clippy, workspace all-targets check pass
- log: `docs/logs/2026-08/03-event-loop-flush-deferral-and-timer-wakes.md`

## Next Task

Promote Card 140.
