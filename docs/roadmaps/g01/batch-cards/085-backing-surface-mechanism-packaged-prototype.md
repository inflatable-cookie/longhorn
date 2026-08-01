# 085 Backing-surface Mechanism Packaged Prototype

Status: complete
Owner: Tom
Roadmap: g01.013 batch 3
Governing refs: contracts 001, 003, 010, 012, 013, and 017; research memo 017
Depends on: Card 082
Auto-start next card: no
Completed: 2026-07-31

## Objective

Apply the frozen coordination prototype through a macOS native backing view
beneath a transparent webview. Prove viewport clipping, scale, forwarded-input
gating, destroy, and selected detach policy without moving Jetstream renderer
authority into Longhorn.

## Scope

- private backing-surface adapter prototype
- packaged product-neutral macOS Tauri proof app
- consumer-supplied native backing view and deterministic test renderer
- full-host native backing geometry
- semantic viewport as render and interaction clip
- transparent renderer hole with explicit viewport measurement
- renderer-forwarded pointer semantics through an injected consumer callback
- viewport move, resize, collapse, restore, and scale evidence
- focus, visibility, host resize, destroy, and detach policy
- Jetstream-shaped trace without scene, camera, picking, or gizmo types
- explicit target support ledger

## Mechanism Behavior

The adapter may keep native backing storage at full host size. Desired viewport
updates become clip and interaction-region updates, not child-frame moves.
Forwarded input is admitted only inside the current viewport and while desired
visibility and focus policy allow it.

The consumer creates the surface, owns rendering, and defines semantic input.
The adapter coordinates native attachment, host evidence, clipping callbacks,
and teardown receipts.

## Out Of Scope

- WGPU device or surface ownership
- render loop, scene graph, camera, picking, gizmo, or frame scheduling
- generic pointer-event payloads
- private Poodle DOM inspection or copied visual primitives
- child-webview or isolated-window adapters
- production package publication
- Jetstream repository migration

## Steps

1. Freeze the Card 082 backing-surface trace and adapter port.
2. Build a controlled native backing view and deterministic renderer fixture.
3. Attach it below a transparent webview in a packaged macOS proof app.
4. Keep native storage full-host while applying viewport clip updates.
5. Add renderer viewport measurement and stale-generation rejection.
6. Forward consumer-defined pointer semantics only through the current gate.
7. Exercise viewport move, resize, zero collapse, restore, and host resize.
8. Exercise available scale changes, focus, visibility, destroy, and detach.
9. Audit Poodle/public-DOM limits, native handles, and dependency edges.
10. Record macOS proof and explicit Windows/Linux support status.

## Acceptance Criteria

- packaged macOS backing-view attach and rendering pass
- native backing geometry may remain full-host while viewport clipping moves
- rendering outside the current viewport is absent
- forwarded input outside the viewport or while disabled is rejected
- input payload meaning remains consumer-owned
- zero viewport suppresses presentation without fabricating detach
- stale viewport and native callbacks leave exact state unchanged
- scale and host resize converge from fresh native observation
- destroy invalidates the attach generation before late callbacks
- reversible or process-lifetime detach policy is declared and receipted
- the shared adapter owns no WGPU, scene, camera, picking, or gizmo types
- the graph omits child-webview, isolated-window, plugin, and Poodle adapters
- Windows and Linux are explicitly unsupported unless separately proved

## Evidence Required

- produced packaged macOS application and scripted backing-view trace
- viewport/clip/render screenshots or deterministic pixel evidence
- forwarded-input admission matrix
- host resize, scale, focus, visibility, destroy, and detach matrix
- stale generation and late-callback fixtures
- native-handle, Poodle-seam, payload, and dependency audit
- per-target support ledger
- focused Tauri, Rust, renderer, docs, and Effigy checks

## Stop Conditions

- the shared adapter must own Jetstream renderer or scene behavior
- viewport semantics require moving the full-host native view
- input gating requires generic pointer payloads in the coordination protocol
- transparent composition cannot preserve trusted renderer chrome
- safe lifecycle evidence requires a process-lifetime leak with no declaration
- the adapter depends on child-webview or isolated-window code
- packaged macOS evidence cannot be produced

## Result

The private nested workspace now contains a backing-only adapter, controlled
AppKit runtime, deterministic consumer renderer, and packaged transparent
Tauri proof. The macOS 26.5.2 arm64 run passes 10 executable checks. One
environmental claim remains unmet: the host exposes one 2x monitor, so no
native scale transition was available and none was simulated. Separate pure
1x/2x conversion fixtures pass.

The native root view stays at full-host bounds while viewport move, resize,
zero collapse, and restore change only the render/output and interaction clip.
Deterministic pixel evidence reports zero lit pixels outside every clip.
Consumer semantic input runs only after current physical-point admission.
Visibility and injected host focus gate presentation and input without
fabricating native visibility or focus observations.

Host resize refreshes full storage without rewriting the desired clip. Stale
plans and native callbacks leave exact runtime state unchanged. Host
destruction invalidates callback authority before reversible native removal
and returns a `detached` receipt.

AppKit handles and unsafe code remain in the proof app. The adapter graph has
no WGPU, scene, camera, picking, gizmo, child-webview, isolated-window,
plugin, Svelte, or Poodle edge. Windows and Linux are explicitly unsupported.
The prototype remains private and non-publishable.

## Next Task

Execute ready Card 086. Compare all three mechanisms and choose the exact
production disposition.
