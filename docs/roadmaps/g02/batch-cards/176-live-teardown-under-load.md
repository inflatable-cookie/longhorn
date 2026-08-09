# 176 Live Teardown Under Load

Status: ready
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

## Ready

`prototypes/gpui-composition` is the application. What it still needs is a real
placement sink with real latency, which is this card's first step.

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
