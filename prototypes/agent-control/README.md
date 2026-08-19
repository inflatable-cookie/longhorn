# Agent Control Spike (card 227)

Throwaway proof for card 227 (`docs/roadmaps/g02/batch-cards/227-agent-control-spike.md`),
answering research memo 024's two runtime gaps:

1. A stateless MCP server (rmcp, streamable HTTP) mounts and serves inside a
   running Tauri app.
2. `WKWebView.takeSnapshotWithConfiguration:completionHandler:` returns fresh
   images for unfocused/occluded windows.

## What it is

A single-window Tauri app. The window shows a large counter that ticks once
per second, with the number in `#counter` and a background hue that cycles
with the count, so snapshot staleness is decidable from the image alone.

Inside the app process, a background thread runs a tokio runtime with an axum
server bound to `127.0.0.1:0` (random port). rmcp's `StreamableHttpService`
is nested at `/mcp` in **stateless** mode (`legacy_session_mode: false`), so
no session ids are minted. On startup the app prints the discovery line:

```
agent-control: listening on http://127.0.0.1:<port>/mcp
```

The MCP server exposes two tools:

- `evaluate(js: string)` — runs the JS in the webview via
  `WKWebView evaluateJavaScript:completionHandler:` and returns the result as
  text (`NSString`/`NSNumber` as plain text, other objects via `description`,
  nil — which covers both `undefined` and `null` — as `"undefined"`).
  Failures come back as tool errors (`isError: true`).
- `screenshot()` — `WKWebView takeSnapshotWithConfiguration:completionHandler:`
  with a nil configuration, converted NSImage → TIFF → NSBitmapImageRep → PNG,
  base64-encoded and returned as an MCP **image** content block
  (`ContentBlock::image(.., "image/png")`).

Bridging: tool handlers run on the tokio thread; tauri's
`WebviewWindow::with_webview` dispatches a closure onto the main thread,
where the `WKWebView` call is issued with a `block2::RcBlock` completion
handler; the handler sends the outcome through a `tokio::sync::oneshot`
channel the async tool awaits.

## Run

```sh
cd proof-app/src-tauri
cargo run
```

Then point any MCP client that speaks streamable HTTP at the printed URL
(e.g. a `.mcp.json` entry `{ "type": "http", "url": "http://127.0.0.1:<port>/mcp" }`).

## Version evidence (pinned in Cargo.lock)

- tauri `2.10.3` (`=2.10.3`, features `wry` only — `with_webview` needs no
  `unstable` feature), tauri-runtime `2.10.1`, tauri-runtime-wry `2.10.1`,
  wry `0.54.4`, tao `0.34.8`. Note: tauri 2.10.3 declares
  `tauri-runtime = "2.10.1"` (caret), which drifts to 2.11.x on a fresh
  lockfile and no longer compiles (the `new_window_handler` box lost its
  `Sync` bound); the lockfile pins `tauri-runtime`/`tauri-runtime-wry` back
  to 2.10.1 via `cargo update --precise`.
- rmcp `3.1.3` (features `transport-streamable-http-server` + defaults
  `server`, `macros`, `base64`).
- objc2-web-kit `0.3.2` (features `WKWebView`, `WKSnapshotConfiguration`,
  `block2`), objc2-foundation `0.3.2`, objc2-app-kit `0.3.2`, objc2 `0.6.4`,
  block2 `0.6.2`.
- axum `0.8.9`, tokio `1.53.1`, base64 `0.22.x` (direct), schemars `1.2.2`
  (via rmcp's re-export, used for tool-arg schemas).

## Accessor path to WKWebView

`WebviewWindow::with_webview` exists in tauri 2.10.3 without any `unstable`
feature: `tauri-2.10.3/src/webview/webview_window.rs:2291`, delegating to
`Webview::with_webview` at `tauri-2.10.3/src/webview/mod.rs:1650`, which
downcasts to `PlatformWebview(tauri_runtime_wry::Webview)`. On macOS
`PlatformWebview::inner()` returns the raw `WKWebView` pointer
(`tauri-2.10.3/src/webview/mod.rs:201-202`), which we retain as
`objc2_web_kit::WKWebView`. The closure runs on the main thread
(documented at `webview_window.rs:2235`).

## rmcp protocol-revision facts (from vendored rmcp 3.1.3 source)

- Supported revision constants: `2024-11-05`, `2025-03-26`, `2025-06-18`,
  `2025-11-25`, **`2026-07-28`** (`rmcp-3.1.3/src/model.rs:170-174`); all five
  are in `ProtocolVersion::KNOWN_VERSIONS` (`model.rs:181-188`).
- `ProtocolVersion::LATEST` is **`2025-11-25`** (`model.rs:175`) — 2026-07-28
  is known but not the default.
- The server **does negotiate**: `initialize` runs
  `negotiate_protocol_version` (`rmcp-3.1.3/src/handler/server.rs:318-331`),
  which echoes the client-requested version when it is in the server's
  supported list and otherwise falls back to the server's own
  `protocol_version` (= LATEST) with a warning
  (`rmcp-3.1.3/src/service/server.rs:469-482`). The default supported list is
  all of `KNOWN_VERSIONS` (`handler/server.rs:339-341`), so a client asking
  for `2025-03-26` gets `2025-03-26` back, one asking for `2026-07-28` gets
  `2026-07-28`.
- Stateless mode: `StreamableHttpServerConfig::with_legacy_session_mode(false)`
  — the field was renamed from 0.13's `stateful_mode` (default is `true`,
  `rmcp-3.1.3/src/transport/streamable_http_server/tower.rs:169`). Per the
  field docs (`tower.rs:65-72`), sessions only exist for legacy protocol
  versions; requests negotiating `2026-07-28` are always served statelessly.
  With `legacy_session_mode: false` and no event store, only POST is accepted
  (GET/DELETE → 405, `tower.rs:1512-1522`), and **no `Mcp-Session-Id`
  response header is set** — that header is only inserted on the legacy
  session initialize path (`tower.rs:1910-1916`); the stateless branch
  (`tower.rs:1918` onward) contains no session-id insert.
- In stateless mode a request without an `MCP-Protocol-Version` header is
  treated as `2025-03-26` (`tower.rs:130-155`, doc text on
  `stateless_protocol_metadata_required`, default `false` at `tower.rs:176`).

## Notes for the live probes

- The app does not need focus or activation for either tool: both go through
  the `WKWebView` object directly, not through AppKit event routing. The
  window is created visible; the counter ticks on a page timer.
- `takeSnapshotWithConfiguration:completionHandler:` is public WebKit API
  since macOS 10.13 — no entitlements, no prompts.
- `undefined` and `null` are indistinguishable from `evaluate` (both arrive
  as a nil result); the tool returns `"undefined"` for both.
- Known WebKit caveat to verify in the probes: a minimized or other-Space
  window may be compositor-throttled; the spike records actual behavior
  rather than assuming freshness.
