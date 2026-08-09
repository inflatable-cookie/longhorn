# GPUI Host Adapter

Date: 2026-08-09
Card: 163 (batch 3)
Roadmap: g02.012

## Result

`longhorn-gpui-windowing` — window create/destroy/observe, placement
application, lifecycle events, close handling, quiescence participation,
display facts. 29 tests. Contract 020 amended from what it found, and the
Tauri adapter re-checked against the amended version: unchanged, suite green.

The experiment was to write the adapter against contract 020 as written and
see where it bends. It bends in twelve places. Eight are GPUI's shape and four
are Tauri assumptions the contract had absorbed.

The twelfth only appeared when a real window was opened.

## Shape

- No `gpui` dependency. Every GPUI value crossing the boundary is a plain
  Rust type; `GpuiWindowBackend` is the seam.
- `prototypes/gpui-windowing` binds that seam to `gpui` 0.2.2, and its smoke
  binary opens a real window and drives an apply pass through it. Excluded
  from the workspace, own lock, like every other prototype.
- Behavioural tests run against an in-memory host implementing exactly
  `gpui::PlatformWindow`'s surface — no `set_position`, no `show`, because
  GPUI has neither.

## Decisions

**The adapter withholds capabilities rather than faking them.**
`gpui_host_capabilities` declares Create, Retag, Maximize, Unmaximize, Focus
and Close, and withholds MoveResize, Show and Hide with a stated reason each.
The pure planner already turns a shortfall into an `UnsupportedOperation`
diagnostic, so this needed no new mechanism — the capability-declaration
design survived a second backend intact, which is the clearest thing the card
proved.

**A withheld capability is not the same as an unreached state.**
`GpuiDiagnosticDisposition` splits the planner's diagnostics three ways:
satisfied at creation, already true, or genuinely unsatisfiable. Without it a
GPUI apply reports `MoveResize` unsupported for a window it just placed
correctly, and a caller cannot tell that from a window it failed to move.

**Display facts are refused, not invented.** GPUI's `PlatformDisplay` has an
id, a UUID and logical bounds. Longhorn's `DisplayFacts` requires a scale
factor and a work area, neither optional. So `GpuiDisplayFactsSource` is an
injected seam, and a caller that cannot supply them gets
`GpuiDisplayObservation::Unobtainable` carrying the list of what is missing —
with the bounds and identity GPUI *does* report still attached. Contract 020
says absence of evidence is recorded as absence; this is what that looks like
in a type.

**The host seam is not `Send + Sync`, and that is now contract.** Every port
in the Tauri host is `Send + Sync`, the host is an `Arc` with two `Mutex`es,
and flushes spawn onto a blocking pool. GPUI windows are reachable only
through `&mut App` on the main thread. This settles Card 161's open question
about the seven pure port types: they stay where they are, because their
thread-safety bounds are host policy, not shared contract. Contract 020 now
has a "What A Host Owns" section saying so.

**Desired state reaches the adapter as a parameter, not from the input.**
GPUI takes bounds, maximized state, focus and display as creation-time options
and cannot change the first two afterwards, so the adapter must know a
window's final placement before the window exists. `WindowDiffInput` shows
desired state only to the planner, because Tauri can mutate after creating and
never had to ask. Making it public is the right fix, and it was made and
reverted: `crates/longhorn-windowing` sits inside the source set the
greenfield receipt freezes at `b7c719c0`, so a one-line visibility change
turns `proof:artifacts` red, and regenerating that receipt belongs to the
release runway another thread is mid-flight on. So
`execute_gpui_window_apply` takes `desired_windows` beside the input that
already contains them, with the reason recorded at the call site. A release
freeze deferred this, not a technical obstacle.

**The `MoveResize` split is scheduled, not taken.** GPUI has `resize` and no
move, so a compound capability forces it to withhold both — a GPUI window
cannot be resized from a plan even though `Window::resize` exists. Splitting
the capability touches the pure planner, `WindowOperation`, and both
adapters. It is the one amendment that would break Tauri if taken carelessly,
so it is recorded in contract 020's divergence register with its consequence
and left for its own card. That is the card's stop condition working as
intended: state the divergence, do not force either backend to match.

**Readback is evidence, not a verdict — and only a real window said so.**
`set_maximized(true)` returns success and the next `is_maximized()` still
reports `false`, because macOS animates the zoom. The apply engine reads back
and re-plans unconditionally, so on GPUI it reports an unconverged maximize
that in fact succeeded, and a caller that trusted convergence would retry
forever. The in-memory host could not have found this: a fake that returns
what it was just told is exactly the fake anyone would write. Recorded in
contract 020 as a readback-semantics change and scheduled, not taken here.

## Evidence

- `crates/longhorn-gpui-windowing` — 29 tests, one module per contract-020
  requirement, refusals asserted rather than skipped
- `prototypes/gpui-windowing` — compiles against real `gpui` 0.2.2; its smoke
  binary opened one window at the exact planned origin, resolved display facts
  once a scale was supplied, and found bend 12
- contract 020 — divergence register, "What A Host Owns", and a per-backend
  current-state table
- `effigy qa` green, including all twelve artifact proofs

## What This Does Not Prove

Contract 020 is **not** complete. Display facts with scale factors are
unsatisfiable from a GPUI host alone. Platform directories are unexercised on
GPUI. No backend has proved multi-window placement, cross-window transfer, or
lifecycle teardown under load, and the first GPUI target — a small
audio-conversion application — exercises none of the three. Those are where a
contract compiled from one host is most likely to have leaked, and the
contract says so in its own Evidence section.

One GPUI window has been opened by Longhorn, on one machine, in one scripted
pass. That is enough to have found bend 12 and not enough to call the adapter
proved at runtime.
