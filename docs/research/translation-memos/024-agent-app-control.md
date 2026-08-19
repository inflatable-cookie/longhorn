# 024 Agent App Control

Status: draft, pre-compilation
Owner: Tom
Updated: 2026-08-19
Promotes: contract 022 (draft). Touches contracts 006 (command invocation),
010 (IPC boundary), 012 (new crates), 020 (host adapter seam).

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

### rmcp serves this today

`rmcp` implements 2026-07-28 statelessly by default (no session ids, no
GET/DELETE, no resumption) while negotiating 2025-11-25 and earlier with
older clients. `StreamableHttpService` is a Tower service mountable on an
axum router inside the app process. Protocol-revision drift is the
library's problem, not the contract's; the contract records "stateless, no
minted sessions" rather than a spec date.

### Unfocused capture is public API

Tauri's `Webview::with_webview` hands over the `objc2_web_kit::WKWebView`
on the main thread. `takeSnapshot(with:completionHandler:)` is public
WKWebView API (macOS 10.13+), renders from the web-content process rather
than the screen, and needs no focus and no screen-recording permission.
Open probe: WebKit may throttle rendering for fully occluded or minimized
windows, which could yield stale snapshots; the spike card must prove
capture freshness for an occluded (not minimized) window and record the
minimized-window behavior honestly.

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

- Occluded/minimized capture freshness — runtime probe, not resolvable
  from documentation.
- Which protocol revisions current agent clients (Claude Code and peers)
  actually negotiate — observe during the spike; affects nothing in the
  contract, only library configuration.
- Mock dialog responders are app-owned; their seam shape is deferred until
  a consumer needs one.
- No provider for native surfaces ships under contract 022; the seam is
  admitted, the first provider needs its own evidence.
