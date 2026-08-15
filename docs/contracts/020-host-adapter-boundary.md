# 020 Host Adapter Boundary

Status: active, amended from second-backend evidence
Owner: Tom
Updated: 2026-08-15
Amended: 2026-08-09 by Card 163, from the GPUI adapter
Architecture: `../architecture/package-topology.md`
Research: `../research/translation-memos/021-dual-backend-host-positioning.md`

## Boundary

Longhorn is a Rust desktop application framework with **pluggable host
backends**. A host provides windows, a lifecycle, and the platform
capabilities Longhorn's pure domain cannot provide for itself. Tauri and
GPUI are both first-class; neither is the reference implementation.

Everything above the host is host-agnostic and is composed identically
whichever backend an application selects.

## Neither Backend Defines The Interface

- The host contract is written from both sides. A requirement is admissible
  only if it can be stated without naming a backend.
- Where the two backends differ irreconcilably, the contract names the
  difference explicitly rather than adopting one shape and making the other
  adapt. A hidden accommodation is worse than a stated divergence.
- Tauri's existence is not evidence that a shape is correct. It is evidence
  that a shape is possible.

## What A Host Must Provide

- **Windows** — create, destroy, and observe, with a stable opaque identity
  Longhorn does not interpret. The identity is the host's to mint. It may be
  a caller-chosen label, or a rendering of a slot index the host allocated;
  nothing above the host recovers structure from it.
- **Placement application** — reach the desired state the plans
  `longhorn-windowing` produces. The planning is pure and shared; execution
  is per-host, and so is **when** state can be applied. A host that can only
  place a window at creation composes creation-time state from desired state
  rather than executing the plan's operation order literally. A host declares
  which operations it can perform on an existing window; the planner turns
  every shortfall into a named diagnostic rather than a failure.
- **Lifecycle events** — created, moved, resized, focus change, close
  requested, destroyed, translated into Longhorn's vocabulary. Translation is
  not required to be one-to-one: a host may report two facts in one callback,
  or none for a fact Longhorn's vocabulary does not carry.
- **Close handling** — a host must let Longhorn observe a close and decide
  whether it proceeds, because restart readiness depends on it. **How** the
  decision is delivered is the host's. A host may demand it synchronously
  inside the callback, and a host that does so has no way to defer and answer
  later. Resumption after a refused close is a per-host path, stated in the
  divergence register below.
- **Quiescence participation** — the host reports its own outstanding work
  to the restart interlock.
- **Display facts** — every display fact the host can report, with its
  identity evidence. Longhorn's display model requires a scale factor and a
  work area; not every host reports either. Where a host cannot, the
  application supplies them and the adapter records which facts were
  unobtainable. An adapter never invents a display fact, and a display whose
  facts are incomplete is still observed.
- **Platform directories** — supplied as values. A host obtains them; it
  does not implement storage.

## What A Host Owns

These are the host's to decide. The contract does not constrain them, and a
requirement written as if one answer were universal is a defect in the
contract.

- **Threading.** A host may deliver events on any thread and may restrict
  window access to one. Host seams are not required to be `Send + Sync`, and
  Longhorn does not require a host to make them so. This is why the pure port
  types identified in Card 161 stay where they are: their thread-safety
  bounds are host policy, not shared contract.
- **When state can be applied.** Creation-time-only state is a legitimate
  host shape, not a deficiency to work around.
- **Whether a window has a visibility state at all.**

## What A Host Must Not Do

- Interpret domain state. A host moves windows; it does not know what a
  panel is.
- Own persistence, entitlement evaluation, update policy, or command
  resolution. Those are host-agnostic and stay so.
- Require a webview. Webview-shaped concepts belong to the webview edge tier
  and are optional.

## Delegated Capabilities

Some capabilities are platform work Longhorn's pure domain cannot perform.
Where a host already provides a mature implementation, Longhorn uses it;
where none exists, Longhorn provides one.

- **Update execution** — download, signature verification, unpack, install,
  relaunch. Tauri hosts use the updater plugin. Non-Tauri hosts use
  Longhorn's native implementation. Both satisfy one behavioural contract
  and one conformance suite; see contract 018.
- **System browser launch** — required by contract 019's RFC 8252 flow.
  Neither backend supplies it: Tauri has a plugin Longhorn does not take, GPUI
  has nothing. So Longhorn implements it once, host-agnostically, in
  `longhorn-browser`, and both hosts compose the same crate.

  This capability hands a server-influenced string to an operating system
  launcher, so it carries two independent defences. `BrowserUrl` is an
  allowlist — HTTPS with a host, ASCII, no control characters, no whitespace,
  no embedded credentials, bounded length. `NativeSystemBrowser` spawns a
  program directly with the URL as a single argument and never involves a
  shell. The usual one-line implementation of this capability, interpolating a
  URL into `sh -c`, is a remote code execution path; neither defence here
  depends on the other holding.
- **OS notification delivery** — unimplemented on both hosts; recorded, not
  promised.

A capability with two implementations carries a shared conformance suite.
Two implementations without one is a fork, not an adapter.

## Divergence Register

Stated, not erased. Each row names the backend whose shape caused it.

| Divergence | Tauri | GPUI | Cause |
| --- | --- | --- | --- |
| Move an existing window | yes | **no** | GPUI's `PlatformWindow` has no position setter; bounds are a creation-time option |
| Resize an existing window | yes | yes | both |
| Runtime show / hide | yes | **no** | GPUI windows are on screen from creation until removed |
| Observe visibility | yes | **no** | GPUI has no visibility query |
| Maximize | absolute `maximize`/`unmaximize` | toggle `zoom_window` + `is_maximized` | GPUI; reachable by read-then-toggle, not atomically |
| Maximize is readable after the call | yes | **no**, not in the same turn | GPUI on macOS animates the zoom; observed directly, see below |
| Normal geometry while maximized | **no**, caller retains it | yes, `WindowBounds` carries restore bounds | Tauri; `retained_normal` in the Tauri capture seam is a Tauri workaround, not contract |
| Display position in the global plane | yes | **not via gpui** — every display reports `(0, 0)`; recoverable from the platform with the id gpui exposes | GPUI's macOS backend reads `CGDisplayBounds` and discards the origin |
| Per-display scale factor | yes | **not via gpui** — per-window only; recoverable from the platform with the id gpui exposes | GPUI's `PlatformDisplay` has id, uuid and bounds |
| Display work area | yes | **no** | as above |
| Built-in display status | unknown in practice | **no** | as above |
| Stable cross-restart display identity | **no**, correlates by name and geometry | yes, `PlatformDisplay::uuid` | Tauri; this is the one place the second backend is stronger |
| Scale-change event | dedicated `ScaleFactorChanged` | carried inside `on_resize` | GPUI; one native event becomes two Longhorn events |
| Close decision timing | may defer and decide later | must answer inside the callback | GPUI |
| Close resumption | host prevents; product policy closes later | refusal returns `false`; the user retries | both, differently |
| Host seam threading | `Send + Sync`, `Arc` + `Mutex`, flushes on a blocking pool | main thread only, `&mut`, no interior mutability | both, differently |

### Readback is evidence, not a verdict

A host adapter that re-observes immediately after applying, and re-plans from
what it sees, assumes the platform has finished. On GPUI/macOS it has not:
`set_maximized(true)` returns success and the next `is_maximized()` still
reports `false`, because the window server animates the zoom. A convergence
readback taken in the same turn therefore disagrees with an operation that
succeeded, and a caller that trusted it would reschedule that operation
forever.

Observed directly rather than reasoned about — `prototypes/gpui-windowing`'s
smoke binary, macOS 25.5, gpui 0.2.2.

**So a host declares which of its operations settle before it can read them
back**, and convergence stops counting the rest. `DeferredSettlement` carries
the declaration; Tauri's is empty and GPUI's names maximize and unmaximize.
Diagnostics are never dropped: an unsupported operation is a fact about the
host, not a timing artefact, and it does not become true later.

### A display inventory is not automatically a desktop plane

GPUI's macOS backend implements `PlatformDisplay::bounds` as:

```rust
// CGDisplayBounds is in "global display" coordinates, where 0 is
// the top left of the primary display.
let bounds = CGDisplayBounds(self.0);
Bounds { origin: Default::default(), size: /* real */ }
```

It reads the global position and throws it away. Sizes are right; every
display reports an origin of `(0, 0)`.

With one display that is harmless and invisible. With two it is not a
coordinate system: every window would be placed on the primary, and two
displays would produce identical arrangement evidence. Contract 009's
correlation and arrangement machinery assumes displays have positions
relative to one another, and on this host they do not.

So **display position is a fact a host may lack**, alongside scale factor and
work area. A GPUI application supplies it or the adapter records it absent;
it is never inferred from a zeroed origin.

Found by attaching a second screen. One display could not have shown it, and
neither could any fake — which is why the contract's coverage table says what
has and has not been exercised rather than assuming a single-display result
generalises.

### A thin host API is not the same as an absent fact

Two claims in earlier drafts of this contract were wrong, in the same way, and
the correction matters more than either finding.

GPUI's `PlatformDisplay` reports an id, a UUID and a size. No scale, no
origin, no work area. From that it looked as though a GPUI application could
not know a display's scale until it had put a window there, and could not know
where a display sits at all. Both were overstatements: **the facts are absent
from the host API, not from the platform**, and GPUI hands over the key to
reach them. `MacDisplay` is a newtype over `CGDirectDisplayID`, and `DisplayId`
exposes it through `impl From<DisplayId> for u32`.

Measured on a two-display desk, with no window open:

| Fact | GPUI reports | Platform reports, same id |
| --- | --- | --- |
| scale, external panel | — | 1 |
| scale, built-in panel | — | 2 |
| origin, external panel | `(0, 0)` | `(0, 0)` |
| origin, built-in panel | `(0, 0)` | `(-1577, 1440)` |

About ten safe lines of `core-graphics` each; see `prototypes/gpui-windowing`.

So `GpuiDisplayFactsSource` is **the seam where a per-platform reader goes**,
not a place to record an impossibility. Its `None` means "no reader supplied
for this platform yet", and the adapter's refusal to invent facts stays right
for exactly that case.

The general rule, which is the part worth keeping: a host abstraction being
thin is a statement about the abstraction. Before recording a fact as
unobtainable, check whether the host has leaked enough identity to ask the
platform directly — because a contract that says "impossible" when it means
"not wired up yet" will stop someone building the thing that was always
available.

What remains genuinely per-window on GPUI is a window's *own* scale, which is
correct: a window can straddle displays, and only the window server knows
which one is driving it.

### Capabilities name one operation each

A capability that names two operations cannot describe a host that has one of
them. `MoveResize` was such a capability, and GPUI has resize and no move —
so it withheld both, and a GPUI window could not be resized from a plan even
though `Window::resize` exists.

`Move` and `Resize` are now separate, and the planner diffs the axes
independently. A host that has one declares one. Tauri declares both, so
nothing about it changed except that a window which only drifted sideways is
no longer resized back to a size it already had.

## Evidence

- Every host-contract claim is proved against **both** backends, or recorded
  as unproven for the backend that lacks it.
- Single-backend evidence does not close a host-contract claim. A dual-target
  framework proved on one target drifts, and the drift is discovered by
  whichever application converts first.
- A proof application exercising a subset states which subset. Absence of
  evidence is recorded as absence, never as success.

### Current State

**This contract is not complete.** Card 163 built the GPUI adapter and the
amendments above came from it, but the evidence has a stated ceiling.

| Requirement | Tauri | GPUI |
| --- | --- | --- |
| Windows: create, destroy, observe | proved | proved, in-memory **and against real windows, including two at once** |
| Placement application | proved | origin proved at creation, on a real window at the exact requested origin; size proved on existing windows; moving an existing window is refused and named |
| Lifecycle events | proved | proved for every event in the list, in-memory only |
| Close handling | proved | proved in-memory, **including thirteen windows torn down out of order with a failing sink**; the real close path ran in the smoke binary |
| Quiescence participation | proved | proved, in-memory, **and that it returns to quiet after a teardown** |
| Display facts with scale factors | proved | **not obtainable from the gpui API alone** — scale, work area and position come from a per-platform reader over the id gpui exposes; a macOS reader exists and was measured against two real displays |
| Platform directories | proved | not exercised |
| Cross-window transfer | proved | **proved live** — a real drag, posted through the window server, resolved to the other window in both directions and to an empty display off them |

All three claims this paragraph used to carry are now discharged in-memory,
and the drag ceiling this paragraph then named is closed too — see "The drag
crossed — 2026-08-10" below. One ceiling remains: **no real teardown has run
with a real flush in flight** (g02.015's Card 176).

The three claims themselves:
Multi-window placement was proved against two real windows across two displays
by `prototypes/gpui-windowing`'s multiwindow binary, and the table above has
said so since; this prose was stale.

### Cross-window transfer, proved for the decision — 2026-08-09

The host's whole contribution to a cross-window drag turned out to be one
thing: where every managed window currently is.
`TransferCoordinator::attempt_target_resolution` takes `&[LiveTransferWindow]`
and decides the rest, so a backend that can observe its windows can
participate.

`longhorn_gpui_windowing::live_transfer_windows` is fifty lines. A point inside
the target window resolves to that window's zone rather than to the source; a
point on bare desktop between two windows resolves to an empty display. Both
run against geometry the GPUI host observed, through the same coordinator the
Tauri host uses.

Two things worth stating from building it.

**The Tauri transfer adapter is 2,600 lines and almost none of it is the
transfer decision.** It is the webview command surface — invoke handlers,
projections, caller authority. A GPUI application calls Longhorn directly and
has no IPC boundary to police, so it needs none of that. The size difference
between the two adapters is not a gap in the GPUI one.

**A window that cannot be observed fails the whole list.** A silently short
list resolves a drop against a desktop missing a window, which reads as "no
target" and loses the transfer with no diagnostic. That is a decision, not an
oversight, and it is asserted.

### Teardown under load found three defects — 2026-08-09

Lifecycle teardown under load is now proved in-memory for the GPUI host:
thirteen windows, each with a pending capture and a staged resize, closed out
of order, with a sink that fails on demand. It went red three times before it
went green, and every failure was in the adapter rather than the test.

**One window's unsaved state blocked every other window's close.** The close
decision read the host's total outstanding work, which is the right number for
the restart interlock — a restart takes every window with it — and the wrong
number for "may *this* window close". A window with nothing to save was
refused because a different window had been moved. Outstanding work is now
tracked per window and the two questions read different numbers.

**A dragged window could never be closed at all.** The capture counter counted
*scheduled deadlines*, and the coordinator debounces by rescheduling the same
pending capture with a fresh deadline. Five move events left five outstanding
captures where only one would ever settle, so the count never returned to zero:
the window could not close and the restart interlock could not read quiet.
Captures and flushes are now pending flags, because a window has at most one of
each in flight.

**A flush that failed to schedule decremented the capture counter.** One
rollback path served both, so a failed flush schedule left its own count raised
and took an unrelated capture down with it. The rollback now names which
counter it raised.

None of the three is visible with one window, and every lifecycle test before
this one used one window. That is the same lesson the display-origin and
readback divergences taught: the evidence found something the moment it got
closer to how the thing is actually used.

### A real store found a window that grew every restart — 2026-08-10

Card 176's first step is a real placement store, so a flush takes real time.
Persisting two real GPUI windows through `ConfigWindowPlacementSink` — a
coordinated, atomically published write, 18-20ms — found a defect that no
in-memory sink could have.

gpui reported `bounds` 560x**592** and `content_size` 560x**560** for a window
asked for 560x560: the 32pt difference is the macOS titlebar.
`capture_from_gpui_facts` recorded the *restore bounds* extent — the outer
height — into `WindowPlacement::inner_size`, which means the inner size.

Applying that placement back makes the window 592 tall inside its frame, so
the next capture records 624, and so on. **A window grew by its titlebar every
save-and-restore cycle.** The function's own comment had described the
approximation and stopped short of its consequence.

Fixed: a window that is not maximized records `content_size`, which gpui
reports and which is the thing the field means. Maximized keeps the restore
extent, because there `content_size` describes the maximized window while the
restore bounds describe where it returns to — that case has no clean answer
and the frame difference is now accepted explicitly rather than by accident.

Both arms are pinned by tests, and re-running the example persisted 560.

This is the pattern again, and the cheapest instance of it yet: the fake sink
was correct about everything except the number, and only a real write to a
real file showed the number was wrong.

### A borrowed context, and what it does to every seam — 2026-08-10

Wiring `GpuiWindowLifecycleHost` into a real application found the most
general difference between the two hosts, and it is not about windows.

**gpui hands out `&mut App` as a borrow that cannot be held.** It is not
`Send`, not `Sync`, and lives only for the callback it arrived in.
`tauri::WebviewWindow` is a cloneable, `Send` handle that a service can keep.

Every Longhorn seam that needs to *see* a window inherits that difference:

- `GpuiWindowCaptureBackend::capture` takes no host context, so a GPUI capture
  backend cannot observe. It must be **fed** facts the application gathered a
  moment earlier, where it had a context. Tauri's holds a window handle and
  fetches.
- `GpuiLifecycleScheduler::schedule` takes no context either, and gpui's
  executors need one, so a GPUI scheduler cannot arm its own timer. It records
  deadlines and the application drains them.

Neither is a defect and neither needed an adapter change. Both are the same
fact wearing two hats: a service that must reach the host can be
self-sufficient on Tauri and cannot be on GPUI.

The consequence for an application author is one line and belongs in the
guide: **on GPUI, a lifecycle service is a handle onto state the application
also holds.** `GpuiWindowLifecycleHost` takes ownership of its services and
exposes no way back to them — correct for services that need nothing — so the
shared half goes in an `Rc<RefCell<..>>` that both sides keep.

`Window::on_window_should_close` is where gpui hands the context back, so the
entire close decision — observe, capture, flush, answer — happens inside it.
There is nowhere else to put it, which is a sharper version of what this
contract already records about GPUI answering closes synchronously.

### The drag crossed — 2026-08-10

A real gesture, posted through the macOS window server with `CGEventPost`, and
resolved by the same `TransferCoordinator` both hosts use. Three cases, all
correct:

```text
released at 1040,368 -> window:1      source window:0
released at 720,918  -> bare desktop
released at 400,368  -> window:0      source window:1
```

Real mouse capture, real gpui dispatch, real geometry. Contract 020's last
stated ceiling is closed for cross-window transfer, and the ceiling that
remains is teardown.

It found two things no in-memory proof could, and one of them corrects
something this contract asserted earlier the same day.

**Element-scoped `on_mouse_up` never fires for a cross-window release.** The
cursor is over the *other* window, so gpui's element hit-test fails — even
though macOS correctly routes the event to the source window that captured the
press. With only `on_mouse_up` bound, the press registered and the release
vanished; both windows sat on "dragging" forever. `on_mouse_up_out` is where a
cross-window release actually arrives, and an application that binds only the
first has a drag that silently never completes.

**A window cannot be observed from inside its own event callback.** gpui takes
a window out of the application's window map for the duration of its dispatch,
so `observe` on it fails with "window not found". `live_transfer_windows`
fails the whole list when any window fails — correct, since a short list loses
a transfer with no diagnostic — so a release handler that observed *everything*
observed *nothing*.

That corrects the freshness claim written here earlier: observe at release is
right for every window except the source, and impossible for the source. The
handler holds `&mut Window` for its own window, so that geometry comes from
there. Freshness is preserved everywhere it is obtainable and the one place it
is not is now explicit rather than assumed.

Both are the borrowed-context fact again, seen from a third angle: gpui's
context is on loan, and anything Longhorn wants to know about the window
currently dispatching has to come from the caller.

### A real close, and a flush that succeeded by having nothing to do — 2026-08-10

A real click on a real close button, answered by Longhorn inside
`on_window_should_close`:

```text
[lifecycle] observe window:0 failed: window not found
[lifecycle] close window:0 -> Close in 42.791µs:
            [Flushed { .. reason: UserClose, outcome: Succeeded }, UserCloseReported]
[lifecycle] outstanding after close: 0 (this window 0)
```

Three things in four lines.

**The borrowed-context constraint reaches the close path too.**
`on_window_should_close` runs inside the closing window's own dispatch, so
observing *that* window fails exactly as it does during a drag release. The
same fact, now seen in the place it matters most.

**The flush succeeded in 42.8µs, and a real store write costs 15-22ms.** It was
fast because it did nothing: the capture failed, so nothing was staged, and a
flush with nothing staged returns `completed()` without touching the store.

**That is a silent-loss path.** In this run the window had not moved, so the
placement already on disk was correct and nothing was lost. Had it moved, the
sequence would be identical — capture fails, nothing stages, the flush reports
success, the close is permitted — and the final placement would be gone with
no diagnostic anywhere. `close_is_safe` would say yes, because a succeeded
flush is a succeeded flush.

The defect is not in the flush. It is that **an application cannot observe a
window at the moment it closes**, so a capture fed at close time is a capture
that fails. The shape that works is to cache facts continuously, on move and
resize, and let the close read what was already there. The composition example
observes at close and is therefore wrong; it is left that way deliberately,
because the failing log line is the evidence.

This answers the per-window flush question this contract has carried since the
teardown work: a per-window close *does* force its own flush, and forcing it is
not sufficient. What matters is whether anything was staged for it to write.

### A moved window's placement is lost at close — 2026-08-10

This contract has carried a question since the teardown work: a window moved
just before it closes stages its final capture and permits the close with no
flush in that pass, and *whether a per-window close should force its own flush*
was left for a real run to decide.

**The placement is lost.** Measured with a real store and a real close. A
window was moved from y=120 to y=324 — confirmed through the accessibility
tree — its close button was clicked, and:

```text
[lifecycle] close window:0 -> Close in 18.625µs: [Captured { generation: 4 }, UserCloseReported]
[lifecycle] outstanding after close: 1 (this window 0)
[lifecycle] window:0 outstanding after drain: 0
```

Captured, permitted, outstanding back to zero. No flush reached the store and
the file still held y=120. Nothing in the receipt is wrong on its own terms,
which is what makes it dangerous: `close_is_safe` sees a capture that succeeded
and a user close that was reported, and there is no failed action to notice.

#### Correction: this is not the shared coordinator, and Tauri does not have it

An earlier version of this section said the sequence belonged to
`longhorn-windowing` and that a Tauri application had been carrying the same
loss. **Both claims were wrong**, and reading the Tauri host rather than
reasoning about it is what showed that.

`longhorn-tauri-windowing` has `shutdown_flush`: it asks every managed window
for a capture, collects the pending flushes, and writes them as one aggregate
under `WindowFlushScope::ApplicationShutdown`. It also calls
`api.prevent_close()` on **every** user close, so the window does not go away
when the user clicks — product policy closes it later, after that flush has had
its chance.

`longhorn-gpui-windowing` has neither. There is no shutdown path in the crate
at all, and GPUI's close is answered synchronously so the window is gone the
moment the decision returns. The staged placement has no later opportunity.

So the defect is an **adapter gap**, not a coordinator flaw: the GPUI host is
missing the shutdown flush its Tauri counterpart has. That is a smaller and
much more actionable finding than the one first recorded here, and it is only
visible because the two hosts were compared rather than one being generalised
from.

#### Closed

`GpuiWindowLifecycleHost::shutdown_flush` now exists. It asks every installed
window for a capture and then issues **one aggregate** write under
`WindowFlushScope::ApplicationShutdown` — the same two-pass shape Tauri uses,
and for the same reason: the coordinator *schedules* a flush rather than
emitting one, which is right in ordinary operation and useless on the way out,
because the deadline it schedules may never arrive.

It returns a `GpuiShutdownReceipt` with the per-window receipts and the
aggregate outcome kept apart, because they fail independently — a window that
could not be captured is not a store that could not be written, and a caller
about to exit needs to tell them apart. `is_complete()` answers the only
question that matters at that point.

Proved against the real store, same gesture as the loss:

```text
close window:0 -> Close in 26.542µs: [Captured { generation: 4 }, UserCloseReported]
outstanding after close: 1 (this window 0)
shutdown flush: Some(Succeeded), complete=true
```

```text
before   window:0 {"x": 120, "y": 120}     the move was lost
after    window:0 {"x": 120, "y": 324}     the move survived
```

A GPUI application must call it before its last window goes. The composition
example calls it from the close callback, because a GPUI window is gone the
moment `on_should_close` returns `true`; a product with a real shutdown path
calls it there instead.

### One behaviour recorded rather than changed

A window moved just before it is closed takes its final capture during the
close, stages it, reports the user close, and permits the close — with no
flush in that pass and none scheduled. The placement reaches the sink's
staging and its durability then depends on whatever flushes next.

This belongs to the shared coordinator, not to either adapter, so **both
backends have it**. Whether a per-window close should force its own flush is a
contract question and not an adapter bug, so it is written down here and
asserted as-is in `teardown.rs` rather than quietly changed. If it should
change, it changes for both hosts at once.

Most of the GPUI adapter's behavioural evidence comes from an in-memory host
implementing exactly `gpui::PlatformWindow`'s surface. That the surface is the
real one is proved by `prototypes/gpui-windowing`, which binds the seam to
`gpui` 0.2.2.

One real GPUI window has been opened by Longhorn, placed from a shared plan,
observed, maximized and closed — the smoke binary in that prototype. It found
the readback divergence above, which the in-memory host could not have. Run
again with a second screen attached, it found the discarded display origin,
which one display could not have. Both are now regression tests.

That is the pattern worth noting: each time the evidence got one step closer
to a real machine, it found something no fake would have. The drag ceiling
this sentence carried closed on 2026-08-10 (see above); what remains is
teardown with a flush in flight — Card 176.

## Non-goals

- A host abstraction that hides platform differences behind a lowest common
  denominator. Longhorn states differences; it does not erase them.
- Runtime host selection. A host is chosen at composition time.
- Support for a host with no maintained implementation. A backend is
  first-class or it does not exist.
