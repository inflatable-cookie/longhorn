# `@inflatable-cookie/longhorn-svelte`

Private Svelte 5 reactive adapters for Longhorn clients.

The root exports per-instance lifecycle, status, mounted cleanup,
request-keyed optimistic projection, and `windowDrag`. The window action
accepts injected native drag and error functions. It has no Tauri import and
ignores controls, links, opt-outs, modifiers, and non-primary gestures.

Optional capability entry points:

- `@inflatable-cookie/longhorn-svelte/layout`
- `@inflatable-cookie/longhorn-svelte/surfaces`
- `@inflatable-cookie/longhorn-svelte/transfer`
- `@inflatable-cookie/longhorn-svelte/surface-transfer`

Layout state consumes checked documents and an injected dispatcher. It does
not create a layout host endpoint. Renderer state is transient: stop and
destroy clear snapshots, pending projections, timers, sessions, and leases.

The transfer subpath adds `DropZoneLeaseRegistry`. Register target elements
before `start()`. Each publication remeasures every current element and sends
one complete client-local replacement. Resize, action update, action destroy,
and registry destroy republish or release authority. The Tauri host performs
the only projection into screen coordinates.

The Surface-transfer subpath adds `surfaceTransferDrag` and
`surfaceTransferDrop`. Pointerdown arms the exact host session; dragstart only
writes an already prepared two-field payload. Drop supports explicit leased
zones or integer screen-DIP points.
