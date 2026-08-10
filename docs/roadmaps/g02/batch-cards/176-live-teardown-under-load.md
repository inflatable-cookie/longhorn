# 176 Live Teardown Under Load

Status: in progress — close observed and a silent-loss path found; restart case outstanding
Owner: Tom
Roadmap: g02.015
Governing refs: contract 020
Depends on: Card 174
Auto-start next card: no

## Objective

A real window torn down with a real flush in flight.

## Why this exists

Teardown under load is proved in-memory: thirteen windows, closed out of order,
against a sink that fails on demand. That proof found three defects — a
window's close blocked by another window's unsaved state, a dragged window that
could never close, and a rollback that decremented the wrong counter.

What it does not cover is a flush that is genuinely in flight: a real store,
a real deadline, and a close arriving while the write is happening. The
in-memory sink answers synchronously, so the whole class of "the answer arrives
after the window is gone" is untested.

One behaviour is already recorded and unresolved: a window moved just before it
closes stages its final capture and permits the close with no flush in that
pass. Contract 020 records it as a coordinator question rather than an adapter
bug. A real run is what decides whether it is a defect.

## State — 2026-08-10

**Step 1 is done and it already found a defect.** The rest needs a person at
the machine.

### The real store

`prototypes/gpui-composition/src/store.rs` is `ConfigWindowPlacementSink` over
a real `ConfigStore`: coordinated mutation, atomic publish, a file under the
example's own target directory rather than a temporary one — a store that
evaporates on drop cannot answer a question about restarts.

A real write costs **18-20ms**. That is the number this card wanted, and it is
large enough that "the answer arrives after the window is gone" is a real
risk rather than a theoretical one.

`longhorn-windowing-config` needed no change to serve a GPUI application. Its
doc comment claimed it was for Tauri window placements; it takes no host
adapter and never did, and that is now corrected.

### What it found

A window that grew by its titlebar every save-and-restore cycle. gpui reports
`bounds` 560x592 and `content_size` 560x560 for a window asked for 560x560,
and `capture_from_gpui_facts` recorded the outer height into a field meaning
inner size. Recorded in contract 020, fixed, and pinned by tests for both the
windowed and maximized arms.

No in-memory sink could have found it, because the defect was in the number
rather than in the flow.

### Two harness mistakes worth recording

The first version of the persistence loop used `let Ok(..) else { continue }`
and wrote an empty document while reporting success — the same silent-skip
failure `live_transfer_windows` exists to refuse, made by the person who wrote
that refusal. Every failure is now reported.

The store result also goes to stderr, not only to the window. A durability
result nobody can read unless they are looking at a screen is a poor result.

### Step 1 is done, and it found the general shape

`prototypes/gpui-composition/src/lifecycle.rs` runs a real
`GpuiWindowLifecycleHost` over both real windows and the real store: an
`Instant`-based clock, and every close answered through Longhorn via
`Window::on_window_should_close`.

Building it found the most general difference between the two hosts, now
recorded in contract 020 and in the composition guide. **gpui hands out
`&mut App` as a borrow that cannot be held**, so a service that needs to see a
window is self-sufficient on Tauri and cannot be on GPUI. A capture backend is
fed rather than fetching; a scheduler records rather than arming. Both are
handles onto state the application also keeps.

Two pieces of scaffolding were written and deleted rather than left: a
`SLOW_FLUSH` threshold and a domain alias, neither of which anything used.

### A real close, 2026-08-10

Clicked, and answered by Longhorn:

```text
[lifecycle] observe window:0 failed: window not found
[lifecycle] close window:0 -> Close in 42.791µs:
            [Flushed { .. reason: UserClose, outcome: Succeeded }, UserCloseReported]
[lifecycle] outstanding after close: 0 (this window 0)
```

**A flush that succeeded by having nothing to do.** 42.8µs against the 15-22ms
a real store write costs. The capture failed, so nothing was staged, and a
flush with nothing staged returns `completed()` without touching the store.

Nothing was lost here — the window had not moved and the placement on disk was
already right. Had it moved, the sequence would be identical and the final
placement would be gone with no diagnostic. `close_is_safe` would say yes,
because a succeeded flush is a succeeded flush.

**The root cause is the borrowed context, again.**
`on_window_should_close` runs inside the closing window's own dispatch, so
observing that window fails — the same constraint Card 175 found at a drag
release. A capture *fed at close time* is a capture that fails. The shape that
works is to cache facts continuously, on move and resize, and let the close
read what is already there.

The example still observes at close and is therefore wrong. Left that way on
purpose: the failing log line is the evidence, and a card that hides its own
finding is worth nothing.

**The per-window flush question is answered.** A per-window close does force
its own flush, and forcing it is not sufficient. What matters is whether
anything was staged for it to write.

### Remaining

1. Move a window, close it, restart, and confirm the loss the analysis above
   predicts — or find it does not happen and learn why.
2. Close while a real store write is genuinely in flight. That needs a staged
   capture, which needs the caching fix, so it follows step 1.

Both need the example frontmost while a window is moved, which is the part
that has repeatedly failed: a titlebar drag sends it behind another
application.

## Scope

- a real placement sink with real latency
- close arriving mid-flush
- the staged-without-flushing path, observed rather than reasoned about

## Steps

1. Give the example a real store, so a flush takes real time.
2. Move a window, then close it immediately, and observe whether the final
   placement survives a restart.
3. Close while a flush is genuinely outstanding and record whether the close is
   refused, deferred, or permitted with the write incomplete.
4. Answer the recorded coordinator question with evidence: does a per-window
   close need to force its own flush? If yes, it changes for both backends.

## Do Not

- Fix the staging behaviour before observing it. It belongs to the shared
  coordinator, so a change lands on Tauri too, and it should be made on
  evidence rather than on the in-memory reading.

## Acceptance Criteria

- a placement moved immediately before a close survives a restart, or the fact
  that it does not is recorded
- a close arriving mid-flush has a stated, observed outcome
- the coordinator question is answered with a real run behind it

## Evidence Required

- a recorded run, including a restart showing what survived
- a decision on the per-window flush question, with the evidence attached

## Stop Conditions

- the staged-without-flushing path turns out to lose placements on both
  backends, in which case stop and treat it as a defect in the shared
  coordinator rather than finishing this card
