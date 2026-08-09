# GPUI Windowing Prototype

Binds `longhorn-gpui-windowing`'s `GpuiWindowBackend` seam to real `gpui`
0.2.2. Card 163 evidence.

## Why it is outside the workspace

`gpui` pulls several hundred transitive crates and a Metal shader build. Every
`effigy qa` lane — `lint:rust`, `lint:rust:features`, `test:rust`, `docs:rust`
— would pay for it, for one adapter. So this crate carries its own
`[workspace]` and its own lock, like every other `prototypes/` crate, and the
workspace crate depends on no GPUI at all.

Poodle draws the same line: `packages/gpui/adapter` has no `gpui` dependency
and only `packages/gpui/preview` does.

## What it proves

That the seam is satisfiable, and that the capabilities
`longhorn-gpui-windowing` withholds are exactly the ones GPUI lacks:

| seam method | gpui call | note |
| --- | --- | --- |
| `create` | `App::open_window` | bounds, maximized, focus and display are `WindowOptions` |
| `resize` | `Window::resize` | content size only |
| `set_maximized` | `Window::zoom_window` | a toggle, so read first |
| `activate` | `Window::activate_window` | |
| `close` | `Window::remove_window` | |
| `observe` | `Window::bounds`, `viewport_size`, `window_bounds`, `scale_factor`, `is_window_active` | |
| `displays` | `App::displays`, `PlatformDisplay::{id,uuid,bounds}` | no scale, no work area |

There is no `move` row and no `show`/`hide` row. `gpui::PlatformWindow` has
neither.

## The smoke binary

`src/bin/smoke.rs` opens a real window from a shared `longhorn-windowing`
plan, observes it, exercises the display-facts refusal and its resolution,
drives the maximize toggle, closes the window, and quits. It prints one JSON
receipt.

```sh
cd prototypes/gpui-windowing
cargo run --bin smoke
```

Recorded 2026-08-09, macOS 25.5, gpui 0.2.2:

```json
{"ok":true,"created":true,"desired_state_reached":true,"dispositions":2,
 "observed_scale":2,"observed_origin":[160,120],"gpui_display_count":1,
 "displays_refused_without_scale":1,"displays_resolved_with_scale":1,
 "maximize_call_ok":true,"maximized_observed":false,"closed":true}
```

Read it as: the window opened at exactly the placement the shared plan asked
for; the display-facts refusal fires on real hardware and resolves once the
caller supplies the scale it learned from a live window; and
`maximized_observed: false` after a successful `set_maximized(true)` is the
finding — macOS animates the zoom, so GPUI's maximized state is not readable
in the same turn as the call.

## The multi-window probe

`src/bin/multiwindow.rs` places two windows from one shared plan across two
displays, and asks whether one scale answer per display holds.

```sh
cargo run --bin multiwindow
```

Recorded 2026-08-09, macOS 25.5, an external panel plus the built-in one:

```json
{"ok":true,"displayCount":2,"windowsCreated":2,
 "multiWindowDesiredStateReached":true,
 "placed":[{"id":"left","origin":[120,140],"scale":1},
           {"id":"right","origin":[900,200],"scale":1}],
 "perDisplay":[{"displayId":2,"primary":true,"windowScale":1},
               {"displayId":1,"primary":false,"windowScale":2}],
 "distinctWindowScales":2,"oneScalePerDisplayHolds":false}
```

Four things came out of it. **Multi-window placement works** — both windows
landed at exactly the planned origins, from a single `plan_window_diff` pass;
contract 020 had this recorded as unproven on either backend.
**`GpuiWindowCreateRequest::on_display` works**, and this is the first time
that path has run. And **one scale per display is false**: 1 on the external
panel, 2 on the built-in. Any implementation that learns a single scale from
a live window and reuses it is wrong by a factor of two on a mixed-DPI desk —
including the one in `smoke.rs`, which is why that binary supplies its scale
to one display only.

And the correction that came out of questioning the third: **none of this
needs a window.** `display_scale_factor` and `display_origin` read both facts
from CoreGraphics using the id GPUI already exposes — `DisplayId` is the
`CGDirectDisplayID`. The run reports `windowlessScales` alongside the
per-window figures, and they match: 1 and 2. The origins do not match, which
is the point — GPUI says `(0, 0)` for the built-in panel and the platform says
`(-1577, 1440)`.

## What it is not

Not a proof application. It runs one scripted pass and exits; it renders
nothing and handles no input. The behavioural tests live in the workspace
crate, against a fake that implements this exact list.

Neither binary can join `effigy qa`: they need a window server, and
`multiwindow` needs two displays to say anything interesting.

## Checking it

```sh
cd prototypes/gpui-windowing
cargo clippy --all-targets
```
