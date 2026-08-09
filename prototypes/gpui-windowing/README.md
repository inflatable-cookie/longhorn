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

## What it is not

Not a proof application. It opens no window and runs no event loop. It is a
compile-time demonstration that the seam matches GPUI's surface. The
behavioural tests live in the workspace crate, against a fake that implements
this exact list.

## Running

```sh
cd prototypes/gpui-windowing
cargo clippy --all-targets
```
