# macOS Mixed-scale Desktop Mapper — 2026-08-17

Card 226. Consumer evidence from Figmatic against the deferred mixed-scale
capability in contract 009.

## What happened

Figmatic could not complete hidden-window restore on a Mac with an external
display: `MixedScaleUnavailable { scales: [1000, 2000] }` from
`UniformScaleMapper`, raised during observation and therefore before restore
planning reads saved state. Clearing saved state cannot recover it.

The refusal was correct. Contract 009 requires an injected platform mapper
before a mixed-scale desktop has a global logical origin, and none existed.

## What was found

macOS composites one logical desktop in points, top-left origin, y down —
`ScreenDip`. The plane does not need deriving; `CGDisplayBounds` and
`NSScreen.frame` report it directly. Measured on the failing arrangement:

```text
DELL U3415W   (0, 0) 3440x1440 pt        backingScale 1.0
Built-in XDR  (-1577, 1440) 1800x1169 pt  backingScale 2.0
```

Reading tao settled the design tension. Tauri's macOS "physical" values are
derived *from* that points plane — monitor position is
`CGDisplayBounds.origin * own scale`, window position is the flipped
`NSWindow.frame` origin times the window's scale. So per-monitor division
inverts correctly on macOS today, which is exactly why the contract refuses to
bless it: that is one crate's arithmetic, not a platform guarantee, and a
change there would silently move every restored window.

## What landed

`MacOsDesktopMapper` over an injected `NativeDesktopPlane`, with
`AppKitDesktopPlane` as the production reader. Displays come from the platform
plane. Physical facts correlate an observation to its native display on size,
scale, and main status — exactly one match or a typed refusal, identical
displays included.

Windows convert through their own scale. Reading `NSWindow` needs `unsafe`,
which this workspace forbids outright; the conversion is licensed by the
correlation, which runs first and fails on displays before any window is
mapped.

Nine tests over the measured arrangement, the `MixedScaleUnavailable` control,
and a live run whose output matches Core Graphics read independently through
Swift.

## Validation

`effigy qa` green. Live native evidence via
`cargo run -p longhorn-tauri-windowing --example macos_desktop_plane`.

## Limits

Left, right, and above display arrangements were **not executed** — each needs
the displays physically rearranged. Neither was a post-apply readback against a
moved window on a second display. The unit suite covers those shapes against
the measured plane, which is not the same as having run them.

Figmatic adoption is the explicit next task and belongs to the Figmatic thread;
Longhorn was not permitted to edit it from this lane.
