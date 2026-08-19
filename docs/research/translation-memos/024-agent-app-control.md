# 024 Agent App Control

Status: complete and promoted
Owner: Tom
Updated: 2026-08-19
Promotes: contract 022 (active 2026-08-19); the g02.029-032 runway. Touches
contracts 006 (command invocation), 010 (IPC boundary), 012 (new crates),
020 (host adapter seam).

## Prompt

Agents developing consumer apps test them through OS computer use: screen
capture plus synthetic system input. They steal focus and the pointer,
serialize behind one desktop, and make the operator's machine unusable
during long runs. Decide the protocol and boundary for an in-app control
surface that lets an agent drive a running app while it stays unfocused.

## Sources

- MCP specification, revision 2026-07-28 (modelcontextprotocol.io): the
  transports overview and Streamable HTTP binding, read 2026-08-19.
- `rmcp` (official MCP Rust SDK) README, read 2026-08-19.
- Tauri `Webview::with_webview` documentation (docs.rs), read 2026-08-19.
- Prior art surveyed for interface shape: Chrome DevTools Protocol,
  WebDriver/`tauri-driver`, Playwright's agent snapshot model.
- Workspace state at `6bfa9616`: no control surface exists in any crate.

## Findings

### Standard remote-control protocols do not reach macOS Tauri

WKWebView speaks neither CDP nor WebDriver; `tauri-driver` does not support
macOS; Playwright's WebKit driver requires a custom-patched WebKit build.
On Windows, WebView2 exposes real CDP for free; the gap is exactly where
these apps live. So the choice is not which protocol to adopt but what
shape the in-app server takes.

### Implementing CDP is rejected

CDP is Chromium's versioned internal surface. The clients worth attracting
sniff Chromium and demand faithful `Input`/`Network` semantics that
WKWebView cannot provide (trusted input injection, interception).
Playwright's `connectOverCDP` is Chromium-only by design. A partial shim
attracts clients that connect and then fail in subtle ways, and the hard
20% — input fidelity — is precisely the part that cannot be faked. What is
worth stealing from CDP: domain decomposition and pushed observability
events (console, errors, navigation).

### MCP revision 2026-07-28 is stateless by construction

The latest revision removed protocol sessions entirely: no `initialize`
handshake, no `Mcp-Session-Id`, no standalone GET stream. Every call is a
self-contained POST to one MCP endpoint; the response is one JSON object or
a request-scoped SSE stream. Server-initiated interaction is gone (MRTR
embeds input requests in results); change notifications ride the SSE
response of an explicit `subscriptions/listen` request; closing a response
stream is cancellation. The spec mandates `Origin` validation, localhost
binding for local servers, and authentication — the exact posture a local
control port needs anyway.

### rmcp serves this today — with two corrections from the spike

`rmcp` (3.1.3, the current line) supports revisions 2024-11-05 through
2026-07-28 and negotiates for real: it echoes the client-requested
revision when known, else falls back to its own default (`LATEST` is
2025-11-25, not 2026-07-28 — the newest revision is supported but not
default). With `legacy_session_mode: false` the service is POST-only and
mints no session ids; the spike's wire capture shows zero
`Mcp-Session-Id` headers. `StreamableHttpService` is a Tower service and
mounted on an axum router inside the app process without incident
(Card 227). Protocol-revision drift is the library's problem, not the
contract's; the contract records "stateless, no minted sessions" rather
than a spec date.

### A current agent client speaks 2026-07-28, session-free

Claude Code 2.1.235 against the spike server: no `initialize` at all — it
opens with `server/discover`, stamps every request with
`mcp-protocol-version: 2026-07-28`, `mcp-method`, and `mcp-name` headers
and `_meta.io.modelcontextprotocol/*` params, lists both tools, and calls
them. Hand-driven calls missing the SEP-2243 envelope get explicit 400s
naming the missing field. The stateless posture the contract mandates is
what a current client already expects.

### Unfocused capture is public API — and is fresh in every window state

Tauri's `Webview::with_webview` hands over the `objc2_web_kit::WKWebView`
on the main thread. `takeSnapshot(with:completionHandler:)` is public
WKWebView API (macOS 10.13+), renders from the web-content process rather
than the screen, and needs no focus and no screen-recording permission.

Spike evidence (Card 227, `prototypes/agent-control/evidence/`): with the
page counter read off each PNG bracketed by `evaluate` calls, the snapshot
showed the current DOM value for a frontmost, an unfocused-visible, a
fully occluded, and a minimized window alike. Capture freshness does not
depend on window state. Another-Space was not probed (not scriptable
without Mission Control UI scripting). One page-side caveat: WKWebView
coalesces 1 s DOM timers to ~2 s in every state including key, and
`requestAnimationFrame` fires only while the window is key. Snapshots stay
DOM-faithful regardless, but agents cannot rely on rAF-driven visuals
advancing while the app is unfocused, and a ticking page is not a wall
clock — freshness must be decided against the DOM, not elapsed time.

### Semantic snapshots beat pixels as the primary surface

The proven agent-facing model is a role/name/state element tree with
stable refs, plus ref-addressed synthetic events (`click`, `type`, `drag`)
dispatched in-page — no OS pointer, no focus. Statelessness forces the
right ref design: refs are stamped into the live DOM at snapshot time and
resolved against the live DOM on use, so no server-side table exists and
concurrent clients interleave safely. Synthetic events are untrusted;
native hover, OS drag-and-drop, and `isTrusted` are documented out of
scope, and native menus and dialogs are reached through contract-006
command invocation, never by clicking native chrome.

### The boundary is observation and input, never authority

The server holds no app state and adds no semantics; behavior stays behind
existing commands and IPC (contracts 006, 010). Non-webview surfaces
(GPUI, native-content islands) appear in screenshots only, behind a
provider seam a native surface can later implement. Dev-feature-flagged
out of release builds; per-instance bearer token; discovery via a
state-directory file (app id, pid, port, token, schema version) removed on
exit and stale-detectable by dead pid.

## Recommendation

Adopt contract 022 as drafted: `longhorn-agent-control` (core: tools,
discovery, token, provider seam), `longhorn-tauri-agent-control` (Tauri
plugin: server mount, window ops, capture), and a TS shim in `longhorn`
(snapshot, dispatch, evaluate). Compile a short runway: one spike card for
the two runtime probes (occluded-capture freshness; rmcp mount inside a
packaged app), then implementation slices per the contract's required
evidence.

## Gaps

- ~~Occluded/minimized capture freshness~~ — closed by Card 227: fresh in
  every probed window state, DOM-relative (see Findings).
- ~~Which protocol revisions current agent clients negotiate~~ — closed by
  Card 227: Claude Code 2.1.235 speaks 2026-07-28 session-free;
  rmcp 3.1.3 supports it but defaults to 2025-11-25 (see Findings).
- Mock dialog responders are app-owned; their seam shape is deferred until
  a consumer needs one.
- No provider for native surfaces ships under contract 022; the seam is
  admitted, the first provider needs its own evidence.
