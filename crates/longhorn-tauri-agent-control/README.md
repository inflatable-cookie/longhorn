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

The app keeps the returned handle and calls `shutdown()` from its run-event
callback — hook `RunEvent::Exit` (and `ExitRequested` where it fires; a
macOS quit delivers `Exit` alone) — so a clean exit removes the discovery
file. The
host supplies a `CommandBridge` over its own contract-006 registry — the
plugin adds no authority of its own.

Wired tools: the full contract 022 surface on macOS (`snapshot`, `click`,
`type`, `press`, `scroll`, `drag`, `evaluate`, `wait_for`, `screenshot`,
`command`, window ops). Synthetic input is untrusted DOM events: it never
moves the OS pointer, never requires focus, and does not satisfy
`isTrusted` checks. Native hover and OS drag-and-drop are out of scope.
`wait_for` is DOM-relative; no time-only or animation-frame wait exists.
`screenshot` is macOS-only capture through the public `WKWebView` snapshot
API (Card 231), answering typed `Unsupported` elsewhere. Page events ride
`subscriptions/listen` as resource updates on
`longhorn://agent-control/{console,error,navigation}`.
