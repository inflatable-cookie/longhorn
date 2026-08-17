# 226 macOS Mixed-scale Desktop Mapper

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

`MacOsDesktopMapper<N: NativeDesktopPlane>` maps displays from the platform's
own plane and never divides a monitor origin. `AppKitDesktopPlane` is the
production reader; the trait is injected so correlation and refusal are
provable without a display attached.

Physical facts are used for exactly one thing: correlating an observation with
the native display it describes, on physical size, scale, and main-display
status. Exactly one match, or a typed `Provider` refusal — including for two
identical external displays, which these facts genuinely cannot separate.
Contract 009 already set that precedent for main-display attribution. Position
is excluded from the key because it is the value most exposed to a future
change in how the host derives physical coordinates, and a key that drifts
silently is worse than one that is coarse.

Managed windows convert through their own scale rather than being read
natively. Reading an `NSWindow` frame means dereferencing the pointer Tauri
hands out, and this workspace sets `unsafe_code = "forbid"` — a boundary worth
more than the last unit of precision. The conversion is sound because the
display correlation just proved the host derives physical facts from the same
plane, and the guard runs first: a host that changed that derivation fails
correlation on the displays before any window is mapped. One assumption with a
guard in front of it, not two independent ones.

The mapper is macOS-only and exposed directly rather than behind a
platform-selecting constructor, so a consumer on another platform meets the
absence at composition instead of as a runtime refusal.

`UniformScaleMapper` is unchanged. A uniform desktop maps identically through
either, pinned by a test.

## Evidence

- nine unit tests over the measured arrangement, including negative origin,
  input-order invariance, a 2x window landing inside the 2x display's mapped
  bounds, unknown-display refusal, identical-display refusal, and
  uniform-desktop equivalence;
- the `MixedScaleUnavailable` control test, so the fail-closed contract cannot
  weaken unnoticed;
- a live native run on the genuinely mixed-scale desktop
  (`cargo run -p longhorn-tauri-windowing --example macos_desktop_plane`),
  whose output matches Core Graphics read independently through Swift.

## Unexecuted arrangements

Recorded rather than claimed. The live run covers one real mixed-scale
arrangement: external 1x main, built-in 2x below and to the left. **Left,
right, and above arrangements were not executed**, and neither was a
post-apply readback against a moved window on a second display, because each
needs the displays physically rearranged. The unit suite covers those shapes
against the measured plane; that is not the same as having run them.

## Acceptance Criteria

- [x] a mixed 1x/2x desktop maps to one plane with no per-monitor division
- [x] correlation failure and ambiguity fail typed rather than approximating
- [x] `UniformScaleMapper`'s fail-closed contract is unchanged and pinned
- [x] the coordinate policy is stated in contract 009 and the architecture note
- [x] native evidence from a genuinely mixed-scale desktop
- [ ] left/right/above arrangements — **not executed**, see above

## Stop Conditions

None fired. The pause signal was whether native macOS APIs could correlate
physical and logical geometry without a new identity or host-authority
boundary; they can, using facts both sides already state.
