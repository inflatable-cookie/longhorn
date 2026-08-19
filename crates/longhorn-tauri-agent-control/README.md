# longhorn-tauri-agent-control

Tauri host wiring for the Longhorn agent app-control surface
([contract 022](../../docs/contracts/022-agent-app-control.md)). Mounts the
host-agnostic `longhorn-agent-control` MCP server inside a running Tauri
application and implements its `ControlHandler` against the app's windows.

The entire surface sits behind the off-by-default `dev` cargo feature. A
build without the feature compiles to an empty library: no server, route,
token, or discovery code exists in the artifact, and no runtime toggle can
enable it. `effigy check:agent-control-release-absence` proves both
directions; the release gate runs it through `effigy qa`.

Composition, from the app's `setup` closure:

```rust,ignore
#[cfg(feature = "dev")]
let agent_control = longhorn_tauri_agent_control::mount_agent_control(
    app.handle(),
    longhorn_tauri_agent_control::AgentControlConfig::new("com.example.app"),
    std::sync::Arc::new(my_command_bridge),
)?;
```

The app keeps the returned handle and calls `shutdown()` from
`RunEvent::ExitRequested` so a clean exit removes the discovery file. The
host supplies a `CommandBridge` over its own contract-006 registry — the
plugin adds no authority of its own.

Wired tools: `command`, `list_windows`, `resize_window`. Snapshot, input
dispatch, `evaluate`, and `wait_for` answer the core vocabulary's typed
`Unsupported` until g02.032; `screenshot` is macOS-only capture through the
public `WKWebView` snapshot API (Card 231), answering typed `Unsupported`
elsewhere.
