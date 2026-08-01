# longhorn-tauri-native-content-child-view

Tauri child-view execution for `longhorn-native-content` plans.

## Boundary

The crate owns:

- one generation-checked child attachment per adapter
- isolation of Tauri's unstable `Window::add_child` API
- physical child bounds, show, hide, focus request, close, and fresh bounds
- exact partial execution receipts through current coordinator authority
- stale callback rejection, explicit host invalidation, and idempotent teardown

Consumers inject the source URL, navigation decision, data-store identity,
logical-to-Tauri label mapping, capability configuration, and bounded native
policy hooks. The optional initialization script is limited to 64 KiB and
never enters the renderer protocol. Page-load, denied-popup, denied-download,
and supported document-title events return only through the native observer.
Popup and download creation remain denied by the supplied Tauri runtime.
Remote content receives no capabilities unless the app adds a matching remote
capability.

The crate does not own navigation products, browsing history, permissions,
downloads, popups, page content, outer-window placement, Svelte, or Poodle.
It imports no isolated-window, plugin, GPU, or backing-surface package.

## Observation

Fresh native child bounds are reported as `ObservedGeometry::ChildBounds`.
Portable effective visibility and focus remain `unknown`; successful show,
hide, or focus calls do not fabricate observation. A finished page load marks
only the adapter's declared readiness condition.

Tauri exposes no portable child-webview input-disable operation. The adapter
therefore applies `native_direct` and returns the exact `child:input-mode`
failure for a visible `disabled` request instead of claiming a gate it did
not enforce. Consumers that require disabled input must hide the child or
inject a stronger target runtime under a later proved contract.

Renderer unmount does not close native content. Desired detach, explicit
teardown, or host destruction ends the attachment. A successfully closed or
invalidated generation cannot be reused.
