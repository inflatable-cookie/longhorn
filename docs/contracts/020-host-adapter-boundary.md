# 020 Host Adapter Boundary

Status: active, amended from second-backend evidence
Owner: Tom
Updated: 2026-08-09
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
| Cross-window transfer | proved | **proved for the decision** — windows observed, a drop resolved to the other window, a bare-desktop point resolved to an empty display; no real drag on either backend |

All three claims this paragraph used to carry are now discharged in-memory,
and one ceiling remains: **no real drag has crossed a real window on either
backend**. A GPUI application binding mouse events to a session, dragging under
the cursor and releasing over another window is still the evidence nobody has,
and this contract is not complete until a target produces it.

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
to a real machine, it found something no fake would have. This contract's one
remaining ceiling — that no real drag has crossed a real window on either
backend — should be read in that light.

## Non-goals

- A host abstraction that hides platform differences behind a lowest common
  denominator. Longhorn states differences; it does not erase them.
- Runtime host selection. A host is chosen at composition time.
- Support for a host with no maintained implementation. A backend is
  first-class or it does not exist.
