# Jetstream Migration Conformance Closeout

Date: 2026-08-02
Roadmap: g01.016
Card: 124
State: complete; Card 125 ready

## Result

Jetstream closes on one window, one native-content island, and the backing-
surface mechanism. Its exact graph is six Longhorn TypeScript packages and six
Rust crates. Config, settings, display, windowing, layout, Surfaces, transfer,
history, operations, notifications, isolated-window, and child-view packages
remain absent.

Longhorn owns the checked bridge session, command discovery and admission,
physical-key resolution, native-content identity and generation, backing-
surface planning and gates, and Svelte session lifetime. Jetstream keeps
command meaning and execution, the editor payload, NSView and WGPU ownership,
rendering, frames, scene, world, camera, picking, gizmos, semantic input, and
outer-window/webview policy.

The duplicate audit finds no raw `jetstream:state` event, raw viewport
endpoint, generic bridge execution bus, renderer shortcut table, or process-
lifetime native leak. Product-specific typed Tauri input commands remain by
design.

## Artifact And Peer Receipt

- Jetstream: `e9a54daacdec1f5c6573687a5543e9ffb2dae2b0`
- private artifact source: `ec465b2a86fe6fbaef789b3677a8e7288e7df2d2`
- Poodle artifact source: `208532f0d18dcd1683cdef157e370d0ba0f0d3b3`
- Poodle set: `25083fe0c5f1b457572c5cb2eb3e3e88f06ed92f55a700d25a9f22d56492cc69`
- Longhorn TypeScript set: `7f62a7d21889c778803cda687248a9048e497cee80d2c2ceb7fa6957e18b3ce0`
- Longhorn Rust set: `42a1a400a7a6066614273a44d86c1686991c92c5454594efe1000aa1730d65c8`

Jetstream-selected Longhorn and Poodle source paths remain unchanged and clean
relative to those artifact receipts. Broader ongoing Poodle work is outside
the selected five package paths. The admitted renderer resolves one Svelte
5.56.8 runtime and Tauri API 2.11.1. The private Rust graph proves Rust 1.85.0
with Tauri 2.11.5. Package-manager publication remains deferred.

## Rollback

The pre-migration Jetstream source `4df74e756c222a3b207391c44775e5b3148c46dd`
passes 19 Rust unit tests, four IPC tests, and 65 renderer tests in disposable
isolated sibling worktrees. Frozen Poodle workspace dependencies are installed
inside that topology before the renderer suite. The first UI attempt exposed
that missing fixture step; the corrected proof passed.

Rollback uses retained source and locks. It adds no dual write, silent
fallback, or second runtime authority. All temporary worktrees were removed;
live repositories were not changed.

## Validation

`effigy qa:northstar:g01-jetstream-card124` verifies the exact graph, immutable
artifact ids, peer ranges, capability inventory, duplicate removal, retained
authority, rollback receipt, 26 current Rust unit tests, six current IPC tests,
and 67 renderer tests.

Card 123 already proves deny-warnings focused Clippy, renderer and release
Tauri builds. Aggregate Jetstream validation still stops on two pre-existing
denied approximate constants in renderer tests. Neither path changed during
the migration.

macOS remains the production backing-surface target. Windows and Linux return
explicit unsupported outcomes. Live scale-transition code is tested at the
boundary, but packaged host evidence remains unmet.

## Next

Execute Card 125. Build the four greenfield compositions from produced
artifacts without donor vocabulary or accidental optional edges.
