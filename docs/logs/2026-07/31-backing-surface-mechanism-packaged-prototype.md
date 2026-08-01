# Backing-surface Mechanism Packaged Prototype

Date: 2026-07-31
Card: 085
Roadmap: g01.013

## Result

Implemented the private backing-surface adapter over the Card 082
coordination prototype. A controlled AppKit root view stays full-host beneath
a transparent Tauri webview. A proof-only deterministic consumer renderer
applies the current viewport as its output and interaction clip.

The packaged macOS 26.5.2 arm64 run passes 10 checks. The available native
scale-transition check is unmet because the host exposes one 2x monitor. No
scale transition is simulated. Pure 1x/2x adapter fixtures pass. Windows and
Linux are explicitly unsupported by this AppKit proof.

## Matrix

- real native root view attaches below the transparent webview
- full-host storage remains separate from the physical viewport clip
- move and resize change the clip and pixel digest without moving storage
- zero viewport emits no lit pixels and does not detach native storage
- restore resumes deterministic clipped output
- output outside the current clip remains absent
- physical input outside the clip is rejected
- consumer semantic payload runs only after current gate admission
- hidden presentation and injected host-focus loss reject input
- fresh host resize changes full storage without rewriting the clip
- stale viewport plans and native callbacks leave exact state unchanged
- destroy invalidates generation authority before reversible detach

## Boundary

The adapter owns plan execution, clip state, generation and revision checks,
presentation and input gates, observation, and detach receipts. It does not
own rendering, semantic input payloads, or product state. Visibility and focus
observations remain unknown; host focus is consumer-injected input policy.

Raw AppKit handles and unsafe code live only in proof-app `native_macos.rs`.
The selected controlled view supports reversible release. No native handle
crosses the adapter protocol.

The adapter graph imports no Tauri, AppKit, WGPU, scene, camera, picking,
gizmo, child-webview, isolated-window, plugin, Svelte, or Poodle package.
Ordinary controlled HTML/CSS supplies proof chrome. No private Poodle DOM is
inspected and no visual primitive is copied.

The packaged `.app` is 9,184 KiB. Its executable SHA-256 is
`9532e7e53e30b4986481ce5bc81db40d010334c96166e247245f0d044f38d5f5`.
The report, transcript, inventory, and target ledger preserve the proof
without committing build output.

## Validation

- seven deterministic adapter contract tests pass on Rust 1.85
- nested Rust 1.85 workspace check and formatting pass
- strict Clippy passes through the repository toolchain
- Tauri 2.10.3 release packaging produces the macOS `.app`
- 10 packaged checks pass; one unavailable native scale transition is unmet
- focused Effigy evidence and Northstar checks pass
- full repository QA result is recorded after the focused gates

## Next Task

Execute Card 086. Compare the pure seam and all three isolated mechanism
proofs. Choose promote, narrow, retain, or reject before creating production
packages or donor migrations.
