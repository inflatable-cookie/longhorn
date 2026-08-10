# Jetstream Backing-surface Coordination Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 123
State: complete; Card 124 ready

## Result

Jetstream now binds `island:jetstream.editor.viewport` to `window:editor`
through Longhorn's checked backing-surface mechanism. Longhorn owns client
epochs, attach generation, desired/observed state, CSS-to-physical conversion,
clip planning, presentation and host-space input gates, apply receipts, and the
Svelte viewport session. Full-host native storage remains distinct from the
physical viewport clip.

Jetstream keeps the NSView, WGPU surface, render thread, scene, world, camera,
picking, gizmo, and semantic input. The runtime port exposes only an opaque
handle and product-free backing-surface snapshots. The raw `set_viewport`
endpoint and process-lifetime NSView leak are gone.

Renderer remounts negotiate fresh client epochs. Stale epochs are rejected.
The initial surface stays hidden under `page_not_loaded`; visibility, scale,
focus, and renderer-forwarded input arrive through one checked session. Host
destroy invalidates coordination, stops and joins the render thread, drops
WGPU, then removes and releases the native view. GPU attach failure also removes
the view. `JETSTREAM_EDITOR_NO_SURFACE` now exposes an explicitly absent island
instead of bypassing the protocol.

## Evidence

- selected Longhorn source: `3032545b3284d3af7f976a88827bb8c8f5c94513`
- prior Jetstream authority: `2a8afbb749965cdfe295b8b6be77de4ba9e00256`
- Jetstream cutover: `e9a54daacdec1f5c6573687a5543e9ffb2dae2b0`
- fixture: `fixtures/migration/jetstream-card123/backing-surface-coordination-cutover-v1.json`
- verifier: removed 2026-08-10 — Longhorn no longer keeps consumer-aware proofs; the recorded fixture is the retained evidence
- canonical app id: `com.inflatablecookie.jetstream.editor`

The selected graph is six TypeScript packages and six Rust crates. It adds no
config, settings, display, windowing, layout, Surface, transfer, history,
operation, notification, or isolated-window edge. No package was published.

## Platform Boundary

macOS retains the production NSView attach and release-built app path. Windows
and Linux return explicit unsupported outcomes. Live scale-transition code and
automated boundary tests pass, but the available packaged backing-surface host
still exposes one native scale. Cross-scale host evidence remains unmet rather
than inferred.

## Validation

Twenty-six Rust unit tests, six Tauri IPC tests, and 67 renderer tests pass. The
focused editor crate passes deny-warnings Clippy. The renderer bundle and
release Tauri binary build.

Jetstream's aggregate `effigy validate` reaches the workspace Clippy gate and
stops on existing denied approximate constants in
`crates/jetstream-renderer/tests/brdf_burley.rs` and
`crates/jetstream-renderer/tests/postfx_vignette_ca.rs`. Card 123 changes
neither path. Advisory Jetstream/Poodle warnings remain upstream debt.

## Next

Execute Card 124. Reverify the complete Jetstream graph, remove any remaining
generic duplicates, and close retained engine authority and rollback posture.
