# Compose A GPUI Application

Status: checked private adoption guidance
Updated: 2026-08-09
Governing contracts: [020](../contracts/020-host-adapter-boundary.md),
[013](../contracts/013-svelte-and-poodle-adapter-lifecycle.md),
[012](../contracts/012-distribution-and-compatibility.md)

## Why This Matters

[Compose Desktop Systems](system-composition.md) is written for a Tauri host:
handlers, capabilities, a renderer, a Svelte session per mounted host. A GPUI
application has none of those and needs the same domains. This guide is the
path through the same contracts for the other backend.

The line is unchanged — Longhorn owns the mechanism, the application owns the
product policy. What changes is what the host gives you for free, and a
webview gives you a great deal that GPUI does not. Most of this guide is that
list.

## What A GPUI Application Must Supply

Start here, because this is the part nobody warns you about. A webview is a
platform as well as a renderer, and three things it hands over for nothing are
yours to decide:

| Fact | Webview | GPUI application |
| --- | --- | --- |
| request ids | `crypto.randomUUID()` | `HostServices::new_request_id` |
| dates a person reads | `toLocaleString` | `HostServices::format_timestamp` |
| case folding for search | `toLocaleLowerCase` | `HostServices::fold_case` |

`longhorn_core::HostServices` bundles all three, supplied once at composition.
`PlainHostServices` implements it for tests and is named to discourage
shipping: an application that reaches for it is telling its users that dates
look like integers.

Locale is genuinely yours. `fold_case` decides whether search matches Turkish
dotless i, and neither Rust's standard library nor JavaScript's default is
right for every product — which is why Longhorn asks rather than guesses.

## The Seams

Every trait a GPUI application implements, in one list. A seam missing from
here is a seam you would otherwise discover from a compiler error.

| Seam | Crate | What it is |
| --- | --- | --- |
| `HostServices` | `longhorn-core` | request ids, dates, case folding |
| `GpuiWindowBackend` | `longhorn-gpui-windowing` | create, resize, maximize, activate, close, observe, displays |
| `GpuiDisplayFactsSource` | `longhorn-gpui-windowing` | scale, work area, position, builtin status — per platform |
| `GpuiWindowCaptureBackend` | `longhorn-gpui-windowing` | capture one window's placement |
| `GpuiLifecycleClock` | `longhorn-gpui-windowing` | monotonic milliseconds |
| `GpuiLifecycleScheduler` | `longhorn-gpui-windowing` | accept one deadline |
| `GpuiUserCloseHandler` | `longhorn-gpui-windowing` | product policy on user close |
| `WindowPlacementSink` | `longhorn-windowing` | stage and flush placements |

`NoopGpuiUserCloseHandler` exists for an application with no close policy.
Everything else you write.

## Composition Order

Build downward from product authority, as the Tauri guide does. Steps 1, 2 and
the last are shared; the middle is where the hosts differ.

1. register application identity, storage profile, and configuration domains
2. create pure domain authorities and product adapters
3. **supply `HostServices`** — once, before anything that formats or folds
4. **implement `GpuiWindowBackend` over `gpui::PlatformWindow`**, and a
   `GpuiDisplayFactsSource` for your platform
5. **build `GpuiWindowLifecycleHost`** from `GpuiWindowLifecycleServices`
   (clock, scheduler, capture, user close, sink)
6. **project domains with `longhorn-poodle`**, render with `poodle-render`,
   draw with `poodle-gpui-node-backend`
7. install listeners before authoritative snapshots
8. reveal only after required authority is ready
9. flush, stop sessions, release leases, and tear down native owners explicitly

## Windows

`execute_gpui_window_apply(input, registry, backend, displays)` takes a
`WindowDiffInput` and applies the plan. It sets the host capabilities itself
from `backend.can_create()`, so you do not declare them.

Read placement back with `observe_gpui_desktop(backend, registry, source)`.

### What GPUI withholds

`WITHHELD_CAPABILITIES` is three entries, and they are not oversights —
`gpui::PlatformWindow` has no equivalent:

- **Move.** A window can be created at an origin and resized afterwards. It
  cannot be moved. A plan that would move an existing window is refused and
  says so.
- **Show** and **Hide**.

`gpui_deferred_settlement()` names the two operations whose effect GPUI does
not report immediately — maximize and unmaximize — so a post-apply readback
does not reschedule work that already succeeded.

### Display facts are not free

GPUI reports a display's size and discards its origin, and reports scale per
*window* rather than per display. Both matter with more than one screen: two
displays will each claim `(0, 0)`, and one scale answer is wrong for a 1x
external screen beside a 2x laptop panel.

`GpuiDisplayFactsSource` is where you supply what the SDK does not.
`prototypes/gpui-windowing` has a macOS reader over `CGDisplayBounds` and
`CGDisplayMode`, measured against two real displays. Other platforms need
their own; there is no fallback and deliberately no guess.

## Lifecycle And Close

`GpuiWindowLifecycleHost::handle_gpui_event` takes a `GpuiWindowEvent` and
translates it. One thing surprises people: a GPUI resize carries the scale, so
one native resize becomes *two* Longhorn events when a window crosses displays.
Tauri has a dedicated scale event and translates one-to-one.

Close is the sharper difference. GPUI's `on_should_close` wants a boolean
synchronously, so `handle_close_requested` returns a `GpuiCloseDecision` and
the whole decision is taken inside the callback. Tauri calls
`api.prevent_close()` on every user close and lets product policy close the
window later by its own route. Both defer; neither resumption path is the
other's.

A deferred close resumes only when the user asks again. Design your close
policy knowing that.

## Drawing

`longhorn-poodle` projects six domains into `poodle-specs`: notifications,
config, settings, operations, licence and update. It takes no `gpui`
dependency, because it emits specs and two Rust renderers already consume
them.

```text
Longhorn domains -> longhorn-poodle          projection
  -> poodle-render         Spec + Theme -> Node
    -> poodle-gpui-node-backend  Node -> AnyElement
      -> gpui                                pixels
```

`poodle-render` is the component tier — one pure function per component. The
backend interprets the node tree. `poodle-gpui`'s `RenderComponent` is the
older tier that inversion replaces; its `render` returns a two-field handle and
draws nothing.

Project a page of notifications with `project_notification_stack`, not
`project_notifications`: `Toast` is a leaf and `ToastStackSpec` is what
renders.

## Cross-window Transfer

`live_transfer_windows(backend, windows)` observes every managed window and
produces the `Vec<LiveTransferWindow>` that `TransferCoordinator` resolves a
drop against. That is the host's whole contribution.

Observe at release, not at drag start. A snapshot taken when the drag began
resolves against where windows *were*, and a window moved mid-drag is exactly
when a stale answer is wrong.

A window that cannot be observed fails the whole call. A short list resolves a
drop against a desktop missing a window, which reads as "no target" and loses
the transfer with no diagnostic.

## What You Do Not Need

`longhorn-tauri-transfer` is 2,600 lines and almost none of it is the transfer
decision — it is the webview command surface: invoke handlers, projections,
caller authority. A GPUI application calls Longhorn directly and has no IPC
boundary to police. The size difference between the two adapters is not a gap
in the GPUI one.

The same is true of `packages/*`. A GPUI application composes a projection
tier, not a parallel client tier.

## Restart And Quiescence

`GpuiWindowQuiescenceProbe` reports the window host's outstanding work to the
restart interlock. It reads at probe time — a receipt from a second ago is not
an answer to "is it safe to restart now".

The interlock's question is application-wide, because a restart takes every
window. "May *this* window close" is a different question with a different
answer, and conflating them meant a window with nothing to save was refused
its close because a different window had been moved.

## Building

`gpui` is not in the workspace: 757 packages and 3.3 GiB of linked artifacts,
paid by every unrelated Rust selector. Host bindings live under `prototypes/`
and are covered by `effigy check:prototypes`, which runs outside `qa` and
inside the release gates.

## Related

- [Compose Desktop Systems](system-composition.md) — the Tauri path
- [contract 020](../contracts/020-host-adapter-boundary.md) — the authority for
  every claim here, including what remains unproven
- [glossary](glossary.md)
