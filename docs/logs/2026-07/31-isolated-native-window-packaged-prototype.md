# Isolated Native-window Packaged Prototype

Date: 2026-07-31
Card: 084
Roadmap: g01.013

## Result

Implemented the private isolated-window adapter over the Card 082 coordination
prototype. A same-binary disposable helper owns a Tauri native window and a
controlled fake `NSView`. The nested workspace is non-publishable and creates
no production authority.

The packaged macOS 26.5.2 arm64 run passes all 11 checks. Windows and Linux are
explicitly unsupported by this AppKit proof.

## Native Matrix

- helper startup reports progress before readiness
- a real fake `NSView` attaches beneath the helper native window
- host-driven content resize converges at the observed native 2x scale
- the matching native resize echo is suppressed
- child resize remains a generation- and revision-bound consumer proposal
- constrained acceptance converges; rejection leaves exact state unchanged
- child hide, show, focus loss, and resize-hint requests remain explicit
- child close returns cooperative teardown evidence
- recentering stays a `longhorn-windowing` consumer decision
- bounded wait reports timeout without fabricating release
- owner termination reports the observed process exit
- abrupt helper loss becomes terminal for its generation
- a stale prior-generation report is rejected without mutation

## Boundary

The adapter spec contains island identity, host-window identity, and teardown
timeout. It has no outer position, display, or frame operation. The proof
consumer computes recentering with `longhorn_windowing::WindowPlacement` and
passes the result only as helper launch arguments.

Raw AppKit FFI lives in `native_macos.rs`. The fake child returns boolean
superview identity; no native pointer crosses the adapter or JSON channel. The
hidden local controller has an empty Tauri permission set.

The adapter graph imports no Tauri, plugin SDK, Signal, child-webview,
backing-surface, GPU, Svelte, or Poodle package. The proof app adds Tauri and
`longhorn-windowing` at its outer consumer edge. No path loads or unloads
third-party code.

The packaged `.app` is 9,444 KiB. Its executable SHA-256 is
`1b9a8a4aefeb7efc30e533f1811ce0086aeb92db3312de6f7df0f4ef4f9bf327`.
The committed report, transcript, target ledger, and inventory preserve the
proof without committing build output.

## Validation

- six deterministic adapter contract tests pass on Rust 1.85
- nested workspace check, strict Clippy, and formatting pass
- Tauri 2.10.3 release packaging produces the macOS `.app`
- every packaged matrix check passes and the transcript has no proof failure
- focused Effigy evidence and Northstar checks pass
- full repository `effigy qa` passes with the new prototype in the gate

## Next Task

Execute Card 085. Prove the independent full-host backing view, viewport clip,
forwarded-input gate, and declared detach policy. Keep isolated-window,
child-webview, renderer, scene, and Poodle authority out of the adapter.
