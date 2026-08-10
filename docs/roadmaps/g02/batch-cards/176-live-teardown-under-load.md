# 176 Live Teardown Under Load

Status: complete — the loss is demonstrated and traced to a missing GPUI shutdown flush
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

### The fix, 2026-08-10

The silent-loss path is closed at its root: **facts are recorded from each
window's own render, not observed at close.**

`facts_from_window` in the `gpui-windowing` prototype reads every fact
`observe` gathers straight from a `Window` the caller already holds — same
values, same order, same `bounds_state`, different route. A render has
`&mut Window` for its own window and gpui redraws on move and resize, so the
capture cache is fresh without an observation, and the close needs none.

Proved by the log line that stopped appearing:

```text
before  [lifecycle] observe window:0 failed: window not found
        [lifecycle] close window:0 -> Close in 42.791µs: [Flushed .. Succeeded, ..]
after   [lifecycle] close window:0 -> Close in 40.083µs: [Flushed .. Succeeded, ..]
```

The close is still fast and still has no `Captured`, and that is now correct
rather than broken: the window had not moved, so the coordinator scheduled no
capture and there was nothing to write. Before the fix the same output meant
the capture had *failed*. The two are indistinguishable from the timing alone,
which is exactly why the disappearing error line is the evidence.

The rule this produces is general enough for the guide and the contract: **the
window you are inside comes from your `Window`; every other window comes from
the backend.** It is the same rule the drag release needed, arrived at from a
second direction.

### The answer, 2026-08-10

**A window moved just before it closes loses its placement.** Measured end to
end with a real store, a real move, and a real close.

The window went from y=120 to y=324, confirmed through the accessibility tree
rather than by assumption. Its close button was clicked:

```text
[lifecycle] close window:0 -> Close in 18.625µs: [Captured { generation: 4 }, UserCloseReported]
[lifecycle] outstanding after close: 1 (this window 0)
[lifecycle] window:0 outstanding after drain: 0
```

Captured. Permitted. Outstanding back to zero. No flush, and the file still
held y=120.

Nothing in that receipt is wrong on its own terms, which is what makes it
dangerous: `close_is_safe` sees a successful capture and a reported close, and
there is no failed action to notice. The write simply never happened before the
window went away.

**Correction: it is not the shared coordinator, and Tauri does not have it.**
This card first recorded the loss as belonging to `longhorn-windowing` and
therefore to both hosts. Reading `longhorn-tauri-windowing` rather than
reasoning about it showed otherwise.

Tauri has `shutdown_flush`, which asks every managed window for a capture and
writes the collected pending flushes as one `ApplicationShutdown` aggregate. It
also calls `api.prevent_close()` on every user close, so the window survives
the click and product policy closes it later — after that flush has had its
chance.

`longhorn-gpui-windowing` has no shutdown path at all, and GPUI answers the
close synchronously, so the window is gone the moment the decision returns. The
staged placement never gets a second chance.

The defect is an adapter gap: **the GPUI host is missing the shutdown flush its
Tauri counterpart has.** Smaller and far more actionable than the coordinator
change first written here, and visible only because the two hosts were compared
rather than one generalised from.

### Getting here needed two fixes and a guard

**Facts from render, not from the close.** `on_window_should_close` runs inside
the closing window's own dispatch, so observing that window fails. Before this,
the close produced no capture at all and an empty flush that reported success.

**Telling the coordinator about the move.** A fresh cache is not enough: the
coordinator schedules a capture when it is told state changed, so a cache that
was current and a coordinator that was never told still produced a close with
nothing staged. gpui has no move callback to bind, so the example compares
bounds between renders.

**A driver that refuses to fire blind.** Every earlier attempt at this
observation failed because the example dropped behind another application and
posted events landed in the operator's session instead — twice. `drive` now
checks that the target is frontmost and that the point lies inside one of its
windows, both through System Events, and exits non-zero naming the failed
check. Verified by watching it refuse.

That guard is why the observation could be taken at all, and it is the piece
worth keeping.
