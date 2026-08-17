# Mixed-scale Desktop Mapping — 2026-08-17

Card 226. Consumer evidence from Figmatic against the deferred mixed-scale
capability in contract 009.

## What happened

Figmatic could not complete hidden-window restore on a Mac with an external
display: `MixedScaleUnavailable { scales: [1000, 2000] }`, raised during
observation and therefore before restore planning reads saved state. Clearing
saved state cannot recover it.

The refusal was correct. Contract 009 requires an injected platform mapper
before a mixed-scale desktop has a global logical origin, and none existed.

## What landed

`LogicalLayoutMapper`: convert every display and window through its own scale.
About twenty lines.

It is valid on macOS and Linux and not on Windows, and the reason is what the
host means by "physical". macOS lays the desktop out in points and reports a
monitor position as its logical origin times that display's scale; Linux has
the same shape through GTK. Dividing returns the layout exactly. Windows reads
`rcMonitor` straight from the OS — a genuine physical-pixel virtual desktop —
so dividing per monitor puts a 1920-wide gap between a 3840x2160 display at
200% and the 1920x1080 display touching it. Windows keeps the typed refusal
until a mapper that reads its layout exists. Nothing is blocked: the evidence
is macOS.

Contract 009 previously banned per-monitor division outright. That ban was
written for the Windows case and read as universal. It now says what is
actually true, per platform.

## The first attempt, and why it was replaced

The card first shipped a native reader — `AppKitDesktopPlane` over `NSScreen`,
an injected plane trait, and a correlation step matching observations to native
displays — around 760 lines and three `objc2` dependencies.

It produced the same numbers. On macOS the physical facts *are* the logical
layout times each object's scale, so the native read and the division are
arithmetically identical; the native version's own fixture demonstrated it,
every physical value being exactly the measured points times the scale.

It was also riskier. `NSScreen` is main-thread-only, so the reader refused off
the main thread. Had Figmatic's restore run on a worker, the fix would have
swapped one typed refusal for another.

The sequencing is the lesson. The evidence that undercut the premise — reading
how tao derives macOS coordinates — arrived partway through, and the work
continued rather than stopping to re-raise it.

## Validation

`effigy qa` green. Six unit tests over an arrangement measured from Core
Graphics on real hardware (external 1x main, built-in 2x below and left,
negative origin), plus the `MixedScaleUnavailable` control.

## Limits

Left, right, and above arrangements were **not executed**; nor was a post-apply
readback against a moved window on a second display. Each needs the displays
physically rearranged. **No Linux arrangement was executed** — that claim rests
on how the host derives monitor geometry, not on a run.

Figmatic adoption is the explicit next task and belongs to the Figmatic thread.
