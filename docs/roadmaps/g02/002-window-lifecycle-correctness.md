# g02.002 Window Lifecycle Correctness

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 009, 010, and 017; research memo 018
Depends on: none

## Outcome

Remove the event-loop stall, the retag state desync, the reachable install
panic, and the parked-thread wake scheduler from the Tauri window lifecycle
host, and close the small lifecycle races the audit recorded.

## Generation Runway

Second g02 milestone. Bounded to `longhorn-tauri-windowing`; the pure
`longhorn-windowing` coordinator semantics stay fixed.

## Execution Plan

### Batch 1. Event-loop deferral and wake delivery

- [x] [Card 139](batch-cards/139-event-loop-flush-deferral-and-timer-wakes.md)
  defers event-path flushes off the event loop, replaces the parked-thread
  scheduler with cancelable timer wakes, and reports wake delivery failures

### Batch 2. Retag coherence and installation safety

- [x] [Card 140](batch-cards/140-retag-coherence-install-safety-and-race-closure.md)
  migrates coordinator state on retag, validates labels before installation,
  and closes the recorded reveal, retained-normal, resurrection, and
  registry-poison races

## Goals

- [x] no lifecycle directive blocks the Tauri event loop for a flush timeout
- [x] retag preserves pending debounce, capture generation, and removes the
  old-id entry
- [x] `install_window` fails typed on invalid labels with no partial state
- [x] wakes use timers, are cancelable per generation, and failures reach the
  `WindowLifecycleReporter`

## Acceptance Criteria

- [x] `Destroyed` and user-close paths complete without synchronous
  `ticket.wait()` on the event thread
- [x] retag under pending capture/flush delivers under the new id; no
  coordinator entry leaks
- [x] oversized-label installation returns an error and installs nothing
- [x] windowing crate suites, mock composition proofs, and workspace QA pass

## Explicit Non-goals

- pure coordinator protocol changes
- consumer repository edits
- Windows/Linux host changes

## Next Task

Promote Card 141 (g02.003).
