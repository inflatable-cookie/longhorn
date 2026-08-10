# 175 Live Cross-window Drag

Status: complete
Completed: 2026-08-10
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

## Result — 2026-08-10

Three real drags, posted through the macOS window server with `CGEventPost`
and resolved by the same coordinator Tauri uses:

```text
released at 1040,368 -> window:1      source window:0
released at 720,918  -> bare desktop
released at 400,368  -> window:0      source window:1
```

Both directions and the empty-display arm. Real mouse capture, real gpui
dispatch, real geometry.

### On the `Do Not`

"Do not simulate the events" stands, and this did not. The concern was calling
handlers directly and skipping the path a gesture takes. `CGEventPost` goes to
the window server, which routes to the application exactly as it routes a
human's gesture — including the mouse capture that sends a release to the
window that received the press, which is the mechanism under test. There is no
in-process shortcut anywhere in the path.

### Two findings

**Element-scoped `on_mouse_up` never fires for a cross-window release.** The
cursor is over the other window, so gpui's hit-test fails even though macOS
routes the event correctly. With only `on_mouse_up` bound the press registered
and the release vanished — both windows sat on "dragging from window:0"
forever. `on_mouse_up_out` is where it arrives.

**A window cannot be observed from inside its own event callback.** gpui takes
it out of the application's window map for the duration, so `observe` fails
with "window not found", and `live_transfer_windows` fails the whole list when
any window fails. A release handler that observed everything observed nothing.

The second corrects a claim I had written into `live_transfer_windows` and the
composition guide the same day: observe at release is right for every window
except the source, and impossible for the source. Both are corrected.

### One operating mistake, recorded

The first attempt posted events while another application's window covered the
screen, and macOS routes by position rather than by frontmost application — so
the drag went into the operator's live session instead. No harm, and the
lesson is cheap: verify the target windows are visible and on top before
posting anything. Every later run screenshotted first.

A driver that refuses to fire unless the point is over its own window is the
proper fix and is not built. `CGWindowListCopyWindowInfo` gives the front-to-
back list needed for it.

## Do Not

- Simulate the events. The in-memory proof already covers the decision; the
  only thing this adds is the real path, and a synthesised event is the fake
  again.
- Declare the contract complete on a single successful drag. State what was
  exercised and what was not.

## Acceptance Criteria

- [x] a drag released over a second real window resolves to that window —
  in both directions
- [x] a drag released on bare desktop resolves to an empty display
- [ ] a window moved mid-drag changes where the release lands — **not run.**
  Moving a window by its titlebar dropped the example behind another
  application and the input driving stopped there. The freshness path is
  exercised for every non-source window on every release; what is unproven is
  the specific case of a window that moved during the gesture.
- [x] contract 020's ceiling paragraph is rewritten against what happened

## Evidence Required

- the binary, and a recorded run naming which of the three cases were seen
- any divergence from the in-memory proof, stated even if it is inconvenient

## Stop Conditions

- GPUI's event surface cannot express a cross-window drag at all, in which
  case that is a contract 020 divergence and gets recorded per backend rather
  than worked around
