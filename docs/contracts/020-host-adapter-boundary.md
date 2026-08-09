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
- **OS notification delivery** — unimplemented on both hosts; recorded, not
  promised.

A capability with two implementations carries a shared conformance suite.
Two implementations without one is a fork, not an adapter.

## Divergence Register

Stated, not erased. Each row names the backend whose shape caused it.

| Divergence | Tauri | GPUI | Cause |
| --- | --- | --- | --- |
| Move an existing window | yes | **no** | GPUI's `PlatformWindow` has `resize` and no position setter |
| Runtime show / hide | yes | **no** | GPUI windows are on screen from creation until removed |
| Observe visibility | yes | **no** | GPUI has no visibility query |
| Maximize | absolute `maximize`/`unmaximize` | toggle `zoom_window` + `is_maximized` | GPUI; reachable by read-then-toggle, not atomically |
| Maximize is readable after the call | yes | **no**, not in the same turn | GPUI on macOS animates the zoom; observed directly, see below |
| Normal geometry while maximized | **no**, caller retains it | yes, `WindowBounds` carries restore bounds | Tauri; `retained_normal` in the Tauri capture seam is a Tauri workaround, not contract |
| Per-display scale factor | yes | **no**, per-window only | GPUI's `PlatformDisplay` has id, uuid and bounds |
| Display work area | yes | **no** | as above |
| Built-in display status | unknown in practice | **no** | as above |
| Stable cross-restart display identity | **no**, correlates by name and geometry | yes, `PlatformDisplay::uuid` | Tauri; this is the one place the second backend is stronger |
| Scale-change event | dedicated `ScaleFactorChanged` | carried inside `on_resize` | GPUI; one native event becomes two Longhorn events |
| Close decision timing | may defer and decide later | must answer inside the callback | GPUI |
| Close resumption | host prevents; product policy closes later | refusal returns `false`; the user retries | both, differently |
| Host seam threading | `Send + Sync`, `Arc` + `Mutex`, flushes on a blocking pool | main thread only, `&mut`, no interior mutability | both, differently |

### Post-apply readback is not universally meaningful

A host adapter that re-observes immediately after applying, and re-plans from
what it sees, assumes the platform has finished. On GPUI/macOS it has not:
`set_maximized(true)` returns success and the next `is_maximized()` still
reports `false`, because the window server animates the zoom. A convergence
readback taken in the same turn therefore disagrees with an operation that
succeeded, and would schedule it again.

**Readback is evidence, not a verdict.** A host states which of its
operations settle synchronously; a convergence diff over an operation that
does not is a false negative, not a reason to retry. Longhorn's Tauri adapter
reads back and re-plans unconditionally today, which is correct for Tauri and
would be a retry loop on GPUI.

Observed directly rather than reasoned about — `prototypes/gpui-windowing`'s
smoke binary, macOS 25.5, gpui 0.2.2.

### The compound capability

`HostCapability::MoveResize` names two operations. GPUI has one of them. A
host that can resize but not move must withhold the whole capability, so a
GPUI window can never be resized from a plan even though `Window::resize`
exists.

**The capability set should separate `Move` from `Resize`.** That is a change
to the pure planner's vocabulary and to both adapters, so it is scheduled
rather than taken here. Tauri declares both today and would declare both
after, so the split is additive for it. Until then, GPUI's adapter reaches
placement only at creation, and says so per window.

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
| Windows: create, destroy, observe | proved | proved, in-memory **and against a real window** |
| Placement application | proved | proved for creation, on a real window at the exact requested origin; refused and named for existing windows |
| Lifecycle events | proved | proved for every event in the list, in-memory only |
| Close handling | proved | proved in-memory; the real close path ran in the smoke binary |
| Quiescence participation | proved | proved, in-memory |
| Display facts with scale factors | proved | **unsatisfiable from the host alone**; the refusal and its resolution both ran against a real display |
| Platform directories | proved | not exercised |

What no backend has proved: multi-window placement, cross-window transfer,
and lifecycle teardown under load. The first GPUI target is a small
audio-conversion application that exercises config, settings, operations,
notifications, licence and update, and none of those three. They are where a
single-host contract is most likely to have leaked, and this contract must
not be declared complete until a target exercises them.

Most of the GPUI adapter's behavioural evidence comes from an in-memory host
implementing exactly `gpui::PlatformWindow`'s surface. That the surface is the
real one is proved by `prototypes/gpui-windowing`, which binds the seam to
`gpui` 0.2.2.

One real GPUI window has been opened by Longhorn, placed from a shared plan,
observed, maximized and closed — the smoke binary in that prototype. It found
the readback divergence above, which the in-memory host could not have. It is
one scripted pass on one machine, not a proof application, and it is outside
`effigy qa` because it needs a window server.

## Non-goals

- A host abstraction that hides platform differences behind a lowest common
  denominator. Longhorn states differences; it does not erase them.
- Runtime host selection. A host is chosen at composition time.
- Support for a host with no maintained implementation. A backend is
  first-class or it does not exist.
