# Jetstream Bridge, Command, And Keyboard Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 122
State: complete; Card 123 ready

## Result

Jetstream now uses one checked `jetstream.editor` bridge domain for the
engine-owned editor-state projection. The renderer negotiates a fresh session
per mount, installs the listener before its first snapshot, replaces whole
snapshots, recovers sequence gaps by resync, and tears down its listener.
Remount invalidates the prior native session.

One sealed Jetstream registry owns 16 semantic commands, one context, and
eight immutable physical-key bindings. Longhorn supplies discovery, current
availability projection, execution-time revalidation, effective-keymap
resolution, typing/repeat/composition gates, consumption, and platform labels.
Jetstream supplies the facts and maps admitted routes to typed render-thread
work. Asset import carries a checked `path` argument; no generic string
execution endpoint crosses the Longhorn boundary.

Viewport, field, selection, picking, and gizmo IPC remain narrow typed routes.
Jetstream retains renderer, WGPU, frame-loop, world, camera, undo/save,
picking, gizmo, semantic-input, and product execution authority. Card 122 adds
four TypeScript packages and four Rust crates. It adds no config, settings,
display, windowing, layout, Surface, transfer, history, operation,
notification, or native-content edge.

## Evidence

- prior Jetstream authority: `4df74e756c222a3b207391c44775e5b3148c46dd`
- Jetstream cutover: `2a8afbb749965cdfe295b8b6be77de4ba9e00256`
- fixture: `fixtures/migration/jetstream-card122/bridge-command-keyboard-cutover-v1.json`
- verifier: `effigy proof:jetstream-card122`
- canonical app id: `com.inflatablecookie.jetstream.editor`

No package was published.

## Validation

Twenty Rust unit tests, five Tauri IPC tests, and 64 renderer tests pass. The
release renderer and Tauri binary build passes. The focused proof re-runs the
Rust/IPC and renderer suites against the exact cutover and lock receipts.

Jetstream's aggregate `effigy validate` reaches the workspace Clippy gate and
then stops on an existing denied approximate-constant lint in
`crates/jetstream-renderer/tests/postfx_vignette_ca.rs`. Card 122 does not
change that file or its crate. Advisory warnings elsewhere remain upstream
Jetstream/Poodle debt.

## Next

Execute Card 123. Wrap the engine-owned native view in checked backing-surface
coordination and one Svelte viewport session without moving WGPU or semantic
input meaning into Longhorn.
