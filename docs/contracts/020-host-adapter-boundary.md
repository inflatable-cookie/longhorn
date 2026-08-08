# 020 Host Adapter Boundary

Status: active compiled boundary
Owner: Tom
Updated: 2026-08-08
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
  Longhorn does not interpret.
- **Placement application** — execute the plans `longhorn-windowing`
  produces. The planning is pure and shared; only execution is per-host.
- **Lifecycle events** — created, moved, resized, focus change, close
  requested, destroyed, translated into Longhorn's vocabulary.
- **Close handling** — a host must let Longhorn observe and defer a close,
  because restart readiness depends on it.
- **Quiescence participation** — the host reports its own outstanding work
  to the restart interlock.
- **Display facts** — known and observed displays, with scale factors.
- **Platform directories** — supplied as values. A host obtains them; it
  does not implement storage.

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

## Evidence

- Every host-contract claim is proved against **both** backends, or recorded
  as unproven for the backend that lacks it.
- Single-backend evidence does not close a host-contract claim. A dual-target
  framework proved on one target drifts, and the drift is discovered by
  whichever application converts first.
- A proof application exercising a subset states which subset. Absence of
  evidence is recorded as absence, never as success.

## Non-goals

- A host abstraction that hides platform differences behind a lowest common
  denominator. Longhorn states differences; it does not erase them.
- Runtime host selection. A host is chosen at composition time.
- Support for a host with no maintained implementation. A backend is
  first-class or it does not exist.
