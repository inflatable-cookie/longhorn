# 084 Isolated Native-window Packaged Prototype

Status: complete
Owner: Tom
Roadmap: g01.013 batch 3
Governing refs: contracts 001, 003, 009, 010, 012, and 017; research memo 017
Depends on: Card 082
Auto-start next card: no
Completed: 2026-07-31

## Objective

Apply the frozen coordination prototype through a macOS isolated native-window
mechanism. Use a controllable fake native child to prove two-way content-size
negotiation, focus, helper failure, and bounded teardown without importing a
plugin ABI or unsafe third-party unload policy.

## Scope

- private isolated-window adapter prototype
- packaged Tauri native-window proof app
- disposable same-binary helper process
- controlled fake `NSView` child with scripted requests
- consumer-owned outer placement through `longhorn-windowing`
- host-driven and content-driven content-size negotiation
- show, hide, close, resize-hint, focus, and focus-loss evidence
- attach readiness, helper loss, timeout, and process-termination receipts
- re-entrancy and resize-cycle suppression
- Soundcheck-shaped lifecycle trace without Signal or plugin binaries
- explicit macOS-only support ledger unless other targets are separately proved

## Mechanism Behavior

The desired viewport supplies isolated-window content size. Outer frame and
display placement remain `longhorn-windowing` decisions. A child-requested size
is a revision- and generation-bound proposal; consumer policy accepts,
constrains, or rejects it before any durable desired-state update.

The fake child may request show, hide, resize, resize-hint change, and close.
Unsafe third-party unloading is not simulated. The helper reports bounded
teardown or explicit process termination.

## Out Of Scope

- CLAP, VST3, AU, Signal, or real plugin loading
- plugin discovery, audio, MIDI, presets, screenshots, or titlebar policy
- generic process supervisor or operation authority
- child-webview or backing-surface adapters
- production package publication
- Soundcheck repository migration

## Steps

1. Freeze the Card 082 isolated-window trace and adapter port.
2. Build a scripted fake native child with attach and request controls.
3. Build the disposable helper and readiness/progress evidence channel.
4. Attach the fake child to an unstable Tauri native window on macOS.
5. Implement content-size proposal, consumer decision, apply, and observation.
6. Exercise host resize, child resize, recenter, and cycle suppression.
7. Exercise show, hide, focus, focus loss, close, and helper loss.
8. Exercise bounded teardown, timeout, and owner-process termination policy.
9. Audit outer-window authority, native-handle confinement, and graph edges.
10. Record macOS proof and explicit Windows/Linux support status.

## Acceptance Criteria

- packaged macOS fake-child attach and interaction pass
- host- and child-driven resize converge without an update cycle
- every child size request requires current consumer acceptance
- outer placement remains outside native-content coordination
- stale child requests and helper reports leave exact state unchanged
- show/hide/close and focus-loss evidence preserve desired/observed separation
- helper loss becomes an exact terminal failure for its generation
- teardown returns completed, timed-out, or owner-process-terminated evidence
- no generic path calls unsafe plugin unload
- raw `NSView` pointers never cross the adapter boundary
- the graph imports no plugin SDK, Signal, child-webview, GPU, Svelte, or Poodle
- Windows and Linux are explicitly unsupported unless separately proved

## Evidence Required

- produced packaged macOS application and scripted helper trace
- host/content resize and cycle-suppression matrix
- show/hide/close/focus/helper-loss matrix
- stale generation, timeout, and teardown receipts
- outer-placement authority audit
- native-handle and dependency inventory
- per-target support ledger
- focused Tauri, Rust, process, docs, and Effigy checks

## Stop Conditions

- useful proof requires a proprietary plugin or Signal implementation
- content-size negotiation requires outer placement in the shared authority
- helper failure cannot be attributed to one attach generation
- bounded teardown cannot report unresolved native ownership safely
- raw pointers must cross the renderer protocol
- the adapter depends on child-webview or backing-surface code
- packaged macOS evidence cannot be produced

## Result

The private nested workspace now contains a pure isolated-window adapter, a
same-binary process runtime, and a packaged Tauri proof. The macOS 26.5.2
arm64 run passes all 11 checks with a real fake `NSView`, native 2x content
sizing, consumer-admitted child resize, rejection invariance, echo
suppression, show/hide/focus-loss, cooperative close, timeout, owner
termination, helper loss, and stale-generation rejection.

Outer placement remains `longhorn-windowing` consumer authority. Raw AppKit
pointers are confined to one proof source and never enter the adapter or wire
protocol. The adapter graph contains no plugin SDK, Signal, Tauri,
child-webview, backing-surface, GPU, Svelte, or Poodle edge. Windows and Linux
are explicitly unsupported by this proof. The prototype remains private and
non-publishable.

## Next Task

Execute ready Card 085. Prove a full-host backing view with viewport-clipped
rendering and renderer-forwarded input.
