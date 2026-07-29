# 021 Tauri Window Host Composition And Mock Proof

Status: complete
Owner: Tom
Roadmap: g01.004 batch 3
Governing refs: contracts 001, 009, 010, and 012; research memo 008
Auto-start next card: no

## Objective

Prove complete host assembly, capability guidance, failure behavior, and both
simple and multi-window composition before packaged runtime proof.

## Scope

- one reusable host assembly function
- Tauri mock-runtime integration
- fake factory, native dispatcher, clock, and persistence sink
- protected single-window and dynamic multi-window fixtures
- capability calculation and minimal capability-file examples
- predeclared and `workspace-*` window patterns
- initialization, listener registration, shutdown, and teardown receipts
- failure injection across probe, apply, event, sink, and flush paths
- host integration documentation

## Public Behavior

Consumers assemble only the capabilities they need. A Nucleus-shaped host uses
one protected predeclared window and no dynamic factory. A Loophole-shaped host
adds dynamic creation. Neither composition imports layout or Surfaces.

Rust-hosted native mutations do not justify broad renderer permissions.
Capability examples grant only renderer actions the example actually uses and
name dynamic window patterns explicitly.

## Out Of Scope

- packaged native runtime proof
- generated TypeScript clients
- application shell components
- consumer repository migration
- layout, Surface, drag, Svelte, or Poodle behavior

## Steps

1. Add reusable runtime-generic host assembly.
2. Add mock Tauri app and managed predeclared window fixtures.
3. Add fake dynamic factory and multi-window lifecycle fixture.
4. Add fault injection for every host boundary.
5. Add minimal capability examples and composition documentation.
6. Prove teardown, shutdown flush, and repeated initialization.
7. Run the full conformance matrix and reassess Card 022.

## Acceptance Criteria

- real and mock hosts use the same assembly
- simple composition has no dynamic-window requirement
- multi-window composition has explicit protected and dynamic patterns
- failure receipts retain stable id, handle, generation, and stage
- repeated initialization cannot duplicate listeners or managed windows
- teardown is idempotent
- capability examples contain no unrelated broad permission
- package graph remains independent of config, layout, Surfaces, TS, Svelte,
  Poodle, and consumers

## Evidence Required

- Nucleus single-window restore/capture/flush conformance
- Loophole protected-main plus dynamic-window conformance
- Soundcheck minimal restore/reveal/close conformance
- mock-runtime listener and shutdown tests
- complete fault-injection matrix
- capability schema validation
- integration guide link checks
- Rust 1.85 and full Effigy QA

## Stop Conditions

- mock and real assembly must diverge
- simple hosts must implement dynamic creation
- renderer capability scope must broaden for Rust-hosted operations
- a failure path lacks inspectable evidence
- packaged or consumer migration work enters scope

## Outcome

`assemble_tauri_window_host` now builds one runtime-generic host for native and
mock Tauri runtimes. Predeclared windows carry stable identity and optional
retained normal placement. Assembly validates identity and protected-primary
policy before listener installation. Process-local Tauri state makes repeated
initialization return `Reused` without duplicate listeners or managed slots.

The composed host owns apply, event handling, guarded reveal, shutdown, and
teardown. Apply temporarily removes the registry from its mutex, so factory,
native mutation, readback, and reveal calls run without that lock. Registry
ownership is restored after both success and pre-execution failure. Dynamic
creation installs lifecycle handling after registry insertion and before later
operations; receipts expose both stages.

Teardown serializes against apply, performs one aggregate bounded shutdown
flush, deactivates listener targets, and returns `AlreadyTornDown` on repeat.
Tauri supplies no listener-removal handle, so callbacks use weak host
references and become inactive no-ops.

Eight mock composition tests cover Nucleus protected single-window restore,
capture, reveal, shutdown, and repeated initialization; Loophole protected
main plus dynamic workspace creation; Soundcheck minimal two-second close;
initialization, planning, factory, native apply, probe/readback, event, sink,
and flush failures; exact operation evidence; and capability parsing.

Minimal capability examples cover `main` and `main` plus `workspace-*`. They
grant only `core:window:allow-start-dragging`; Rust-hosted native operations do
not broaden renderer authority. The
[integration guide](../../../architecture/tauri-window-host-integration.md)
documents both compositions, services, capability posture, failures, and
teardown.

Current-toolchain warnings-denied Clippy, package tests, link checks, and full
Effigy QA pass. Rust 1.85 remains blocked before compilation by the existing
locked Tauri graph: ICU requires 1.86 and current Darling, plist, serde_with,
and time require 1.88. Dependency-floor reconciliation stays in Card 022.

## Next Task

Stopped at the packaged-proof checkpoint. The native macOS environment and
Rust 1.85 dependency-floor plan are now recorded in Card 022. Explicit
operator start is still required.
