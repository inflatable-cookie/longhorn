# longhorn-tauri-bridge

Narrow Tauri command and event assembly for the generic Longhorn bridge.
Consumers inject domain registrations, typed handlers, and authority.

Build a `BridgeDomainRegistry`, register domain-owned typed routes, then wrap
`BridgeHandlerAssembly` in `TauriBridgeState`. Register exported commands by
their full crate paths:

```rust,ignore
builder
    .manage(TauriBridgeState::new(assembly))
    .invoke_handler(tauri::generate_handler![
        longhorn_tauri_bridge::longhorn_bridge_hello,
        longhorn_tauri_bridge::longhorn_bridge_authority,
        longhorn_tauri_bridge::longhorn_bridge_query,
        longhorn_tauri_bridge::longhorn_bridge_command,
        longhorn_tauri_bridge::longhorn_bridge_cancel,
        longhorn_tauri_bridge::longhorn_bridge_resync,
    ])
```

Use `BridgeHandlerAssembly::new` for invoke-only hosts. Use
`with_event_sink` with `TauriBridgeEventSink` or a mock sink only when the app
needs subscriptions. Tauri capabilities admit calls; negotiated bridge
authority still controls dispatch.

Copy a minimal permission/capability pair from `examples/`.
