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

## What it is not

Not a proof application. It runs one scripted pass and exits; it renders
nothing and handles no input. The behavioural tests live in the workspace
crate, against a fake that implements this exact list.

It cannot join `effigy qa`: it needs a window server.

## Checking it

```sh
cd prototypes/gpui-windowing
cargo clippy --all-targets
```
