# 175 Live Cross-window Drag

Status: in progress — harness built, run not yet performed
Owner: Tom
Roadmap: g02.015
Governing refs: contract 020
Depends on: Card 174
Auto-start next card: no

## Objective

Close contract 020's last stated ceiling: a real drag across two real GPUI
windows.

## Why this exists

Cross-window transfer is proved for the *decision*.
`longhorn_gpui_windowing::live_transfer_windows` observes windows, and the
coordinator resolves a point inside one to that window rather than the source.
What no backend has ever done is bind mouse events to a session, drag under the
cursor, and release over another window.

Contract 020 states this as the one remaining ceiling. The pattern this
generation established is that each step closer to a real machine found
something no fake would: the readback divergence, the discarded display origin,
three teardown defects. There is no reason to expect this step to be different.

## State — 2026-08-10

**The harness is built and the drag has not been performed.** Said plainly
because the difference is the whole point of this card: everything below
compiles and runs, and nobody has yet pressed in one window and released over
the other.

### What exists

`prototypes/gpui-composition` now opens two windows and `src/drag.rs` binds
them:

- `on_mouse_down` starts a `TransferCoordinator` session sourced from the
  window that was pressed
- `on_mouse_up` converts `MouseUpEvent::position` — window-relative — to a
  screen point by adding the window's own origin, then resolves
- windows are observed **at release**, through `live_transfer_windows` over
  the neighbouring prototype's `GpuiAppBackend`, so a window moved mid-drag
  changes where the release lands
- both windows draw the outcome, so either can be watched

Leases are published from **observed** bounds rather than requested ones. The
two agree on this platform, but a lease published from a request is wrong the
first time a window manager disagrees.

### What it already found

Three defects, all mine, all in the harness rather than in Longhorn:

- the windows paint before `install` runs — they must exist before their
  bounds can be observed — so `cx.global` panicked on the first frame.
  `try_global` and a placeholder string.
- a lease lifetime of 900 against a `maximum_lease_lifetime` of 500. The
  coordinator refused it with `InvalidLifetime` and named both numbers, which
  is the error doing its job.
- `screen_rect_of` written and then unnecessary once bounds came from
  observation instead of assumption.

None of these is evidence about contract 020. They are recorded because the
card asks what the real path found, and "the harness was wrong three times
first" is part of an honest answer.

### Remaining

1. Run it with the machine free. Press in one window, release over the other,
   and read the outcome line.
2. Release on bare desktop between the windows.
3. Move one window mid-drag and release into its new position.
4. Record what happened in contract 020, whether or not it agrees with the
   in-memory proof.

Steps 1-3 need a person at the machine: the card's own `Do Not` forbids
synthesising the events, and the point of this card is the real path.

## Scope

- GPUI mouse events bound to a `DragSessionId`
- a release resolved against `live_transfer_windows` at release time, not at
  drag start
- both outcomes: a release over the second window, and a release on bare
  desktop

## Steps

1. Start a session from a press in window A, using the coordinator the
   in-memory proof already uses.
2. Resolve on release, observing windows at that moment. A snapshot taken at
   drag start resolves against where windows *were*, and a window moved
   mid-drag is exactly when a stale answer is wrong.
3. Move one window mid-drag and release into its new position. If the
   resolution follows, freshness is proved rather than asserted.
4. Record what the real path found, in contract 020's current state, whether or
   not it agrees with the in-memory proof.

## Do Not

- Simulate the events. The in-memory proof already covers the decision; the
  only thing this adds is the real path, and a synthesised event is the fake
  again.
- Declare the contract complete on a single successful drag. State what was
  exercised and what was not.

## Acceptance Criteria

- a drag released over a second real window resolves to that window
- a drag released on bare desktop resolves to an empty display
- a window moved mid-drag changes where the release lands
- contract 020's ceiling paragraph is rewritten against what happened

## Evidence Required

- the binary, and a recorded run naming which of the three cases were seen
- any divergence from the in-memory proof, stated even if it is inconvenient

## Stop Conditions

- GPUI's event surface cannot express a cross-window drag at all, in which
  case that is a contract 020 divergence and gets recorded per backend rather
  than worked around
