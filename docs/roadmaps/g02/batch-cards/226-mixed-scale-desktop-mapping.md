# 226 Mixed-scale Desktop Mapping

Status: complete — 2026-08-17
Completed: 2026-08-17
Owner: Tom
Roadmap: g02 (consumer evidence against the completed g01 windowing lane)
Governing refs: contract 009 (§ Tauri adapter, coordinate conversion);
`docs/architecture/tauri-window-host-integration.md`; g01 Card 017
Depends on: none — Card 017 is historical authority, not reopened
Auto-start next card: no

## Objective

A macOS desktop containing 1x and 2x displays maps to one coherent logical
plane, so window placement persistence works on an ordinary laptop-plus-monitor
arrangement.

## Why this exists

Figmatic failed hidden-window restore during startup on a Mac with an external
display:

```text
observe Figmatic desktop failed:
Projection(Mapping(MixedScaleUnavailable { scales: [ScaleFactor(1000), ScaleFactor(2000)] }))
```

That refusal is contract 009 working as designed: `UniformScaleMapper` is valid
only for an established single-scale desktop, and mixed-scale global origins
require an injected platform mapper that did not exist. Observation fails
before restore planning reads saved state, so clearing saved state cannot
recover it. This is fresh consumer evidence against a deferred capability, not
a regression.

## What the evidence established

macOS already composites one coherent logical desktop. `CGDisplayBounds` and
`NSScreen.frame` report points in a single global plane whose origin is the
top-left of the main display and whose y grows down — which is exactly
`ScreenDip`. Measured on the failing arrangement (2026-08-17):

```text
DELL U3415W   (0, 0) 3440x1440 pt        backingScale 1.0
Built-in XDR  (-1577, 1440) 1800x1169 pt  backingScale 2.0
```

The plane is not something to derive. It is something to read.

Reading tao rather than reasoning about it also settled the open tension. On
macOS, Tauri's "physical" values are *derived from* that points plane: a
monitor's position is `CGDisplayBounds.origin * its own scale`, and a window's
is its flipped `NSWindow.frame` origin times the window's scale. Per-monitor
division therefore happens to invert on macOS today — which is precisely why
contract 009 refuses to bless it. It is a property of one windowing crate's
arithmetic, not of the platform.

## Result

`LogicalLayoutMapper` converts every display and window through its own scale.
That is the whole mechanism, and it establishes one coherent plane across mixed
scales. `UniformScaleMapper` is unchanged; a uniform desktop maps identically
through either, pinned by a test.

It is valid on macOS and Linux, and **not** on Windows. Both macOS and Linux
lay the desktop out in logical units and report physical facts as those units
times each object's scale, so dividing returns the layout exactly. Windows
reads `rcMonitor` straight from the OS — a real physical-pixel virtual desktop
— where per-monitor division puts a 1920-wide gap between a 3840x2160 display
at 200% and the 1920x1080 display touching it. Windows keeps the typed
`MixedScaleUnavailable` refusal until a mapper that reads its layout exists.
Nothing is blocked by that: the consumer evidence is macOS.

## The first implementation was wrong, and how

This card first shipped a native reader: an `AppKitDesktopPlane` over
`NSScreen`, an injected `NativeDesktopPlane` trait, and a correlation step
matching each Tauri observation to a native display on size, scale, and main
status — about 760 lines and three `objc2` dependencies.

It computed the same answer. The evidence that showed it was reading tao: on
macOS the physical facts *are* the logical layout times each object's scale, so
division and the native read are arithmetically identical. The native reader's
own test fixture proved it — every physical value in it was exactly the
measured points times the scale.

Worse than redundant, it was riskier. `NSScreen` is main-thread-only, so the
reader refused off the main thread. Had Figmatic's hidden restore run on a
worker, the fix would have swapped one typed refusal for another and the bug
would have survived.

The mistake was not the code but the sequence: the tao evidence undercut the
premise of the task partway through, and the work continued instead of
stopping to say so. The handoff's pause condition asked whether correlation was
*possible*; the better question, once that evidence existed, was whether it was
*necessary*.

Recorded because the deleted design is the reason the surviving one is only
twenty lines, and because contract 009's blanket prohibition is what made the
elaborate version look mandatory. That prohibition is now stated accurately:
division is invalid where the desktop plane is physical, which is Windows, and
valid where it is derived, which is macOS and Linux.

## Longhorn picks the mapper, not the application

The mapper began as a consumer-supplied argument, which meant every one of the
sibling apps had to know the platform rule and edit two call sites to get the
fix — and an app that missed it kept compiling while staying broken on
mixed-scale desktops. Silent wrongness is the worse failure.

Choosing a coordinate mapper was never a product decision. It follows entirely
from the target platform, about which an application knows nothing Longhorn
does not. So `observe_tauri_desktop`, `TauriDesktopReadback::new`, and
`plan_tauri_window_restore` no longer take one; `PlatformDesktopMapper`
resolves at compile time to `LogicalLayoutMapper` on macOS and Linux and to
`UniformScaleMapper` elsewhere. `observe_tauri_desktop_with` and
`TauriDesktopReadback::with_mapper` remain for a test double or an undescribed
host.

This is a consumer break, taken deliberately with operator approval. It costs
one mechanical deletion per call site now — compiler-enforced, so it cannot be
missed or done wrongly — and it buys the thing that matters at this fleet size:
**a platform mapper that lands later reaches every app on the next Longhorn
revision with no application change at all.** A Windows mapper is the next one
that will.

Longhorn's own `examples/tauri-windowing-proof` was still passing
`UniformScaleMapper` and is fixed in the same pass; the proof app had been left
broken on exactly the arrangement this card exists for.

## Evidence

- seven unit tests over the arrangement measured on real hardware, including the
  negative origin, a 2x window landing inside the 2x display's mapped bounds,
  per-object scales in one snapshot, and uniform-desktop equivalence;
- the `MixedScaleUnavailable` control test, so the fail-closed contract cannot
  weaken unnoticed;
- a per-platform assertion on `PlatformDesktopMapper` itself, so the default a
  consumer now silently receives is pinned rather than inferred from the alias.

## Unexecuted arrangements

Recorded rather than claimed. **Left, right, and above arrangements were not
executed**, and neither was a post-apply readback against a moved window on a
second display; each needs the displays physically rearranged. The measured
arrangement — external 1x main, built-in 2x below and left — was read from
Core Graphics on real hardware, and the unit suite is built on those numbers.

No Linux arrangement was executed at all. The Linux claim rests on reading how
the host derives its monitor geometry, not on a run.

## Acceptance Criteria

- [x] a mixed 1x/2x desktop maps to one coherent plane
- [x] `UniformScaleMapper`'s fail-closed contract is unchanged and pinned
- [x] the coordinate policy is stated accurately in contract 009 and the
  architecture note, per platform rather than as a blanket ban
- [x] Windows is named as excluded rather than silently mismapped
- [x] consumers no longer choose a mapper, and the platform default is pinned
- [x] Longhorn's own windowing proof uses the platform default
- [ ] left/right/above arrangements — **not executed**, see above
- [ ] any Linux arrangement — **not executed**, claim rests on host derivation

## Stop Conditions

The stated pause signal — whether native macOS APIs could correlate physical
and logical geometry without a new identity boundary — did not fire; they can.

A different one should have. When the evidence showed the platform already
supplies the plane the task was written to reconstruct, that changed the
premise and warranted stopping to re-raise it rather than building on.
