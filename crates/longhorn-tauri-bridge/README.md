# longhorn-tauri-bridge

Narrow Tauri command and renderer-local event assembly for the Longhorn bridge.
Consumers inject domain registrations, typed handlers, and authority.

Build a `BridgeDomainRegistry`, register domain-owned typed routes, then install
`BridgeHandlerAssembly` through `bridge_plugin` (which wires window-destroyed
session teardown by default). Register exported commands by their full crate
paths:

```rust,ignore
builder
    .plugin(longhorn_tauri_bridge::bridge_plugin(assembly))
    .invoke_handler(tauri::generate_handler![
        longhorn_tauri_bridge::longhorn_bridge_hello,
        longhorn_tauri_bridge::longhorn_bridge_authority,
        longhorn_tauri_bridge::longhorn_bridge_query,
        longhorn_tauri_bridge::longhorn_bridge_command,
        longhorn_tauri_bridge::longhorn_bridge_cancel,
        longhorn_tauri_bridge::longhorn_bridge_resync,
    ])
```

A session belongs to a window, and only its UI webview (the one sharing the
window label) can use the bridge. That holds when the window also hosts child
webviews, such as native-content islands; the children are refused. The crate
enables Tauri's `unstable` feature to do this.

Events reach only sessions with read authority for the domain. Progress and
terminal events also need execution authority.

Use `BridgeHandlerAssembly::new` for invoke-only hosts. Use
`with_event_sink` with `TauriBridgeEventSink` when the app needs subscriptions.
The sink dispatches `CustomEvent` in the owning renderer. Bridge subscriptions
use DOM listeners and require only the invoke permissions in the capability
example. Negotiated bridge authority controls dispatch.

Copy a minimal permission/capability pair from `examples/`.
