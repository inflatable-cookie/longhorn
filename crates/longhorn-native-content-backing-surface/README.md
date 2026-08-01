# longhorn-native-content-backing-surface

Generic backing-surface coordination over consumer-owned native storage and
render execution.

The adapter keeps full-host storage distinct from the semantic viewport. It
applies the viewport as the output and interaction clip, gates physical input
before consumer semantic dispatch, records fresh storage/render evidence, and
rejects stale generations. Host focus is injected gate evidence; it is not
native focus or visibility observation.

Consumers provide native storage, renderer lifecycle, clipping, and reversible
detach through one narrow runtime port. The crate contains no Tauri, AppKit,
WGPU, scene, camera, picking, gizmo, semantic input, raw pointer, Svelte, or
Poodle type.
