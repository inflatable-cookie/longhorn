# Card 227 Probe Results

Date: 2026-08-19. Host: macOS (Apple Silicon), operator's machine.
App: `proof-app` debug binary, window `main` (520×320 at 640,242 on a
1440×900 display), page counter in `#counter` ticking via
`setInterval(1000)`.

Method: every screenshot is bracketed by `evaluate` calls reading
`#counter` immediately before and after (`/tmp/agent-control-probe.py`,
driving the same MCP endpoint through a logging proxy). The number read
off the PNG must equal the bracket — freshness is DOM-relative, not
wall-clock. MCP client probes used Claude Code 2.1.235 registered with
`claude mcp add --transport http` against the wire-tap proxy.

## MCP mount and negotiation

- Claude Code connects, lists both tools, and calls both successfully
  (`wire-capture.log`). It speaks revision **2026-07-28**: first request is
  `server/discover` (no `initialize` on this revision), every request
  carries `mcp-protocol-version: 2026-07-28`, `mcp-method`, and
  `mcp-name` headers plus `_meta.io.modelcontextprotocol/*` params.
- The server's `server/discover` reply lists supportedVersions
  `2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25, 2026-07-28`.
- **No `Mcp-Session-Id` header appears anywhere in the capture** — zero
  minted sessions in stateless mode (rmcp 3.1.3,
  `legacy_session_mode: false`). GET/DELETE are 405 by construction.
- Hand-driven calls (python urllib) against 2026-07-28 were rejected
  until they carried the SEP-2243 headers and `_meta` fields: the
  revision enforces its own envelope. Missing-header errors are explicit
  400s naming the missing header.
- `evaluate` and `screenshot` answered while the app never held OS
  focus: System Events reported `frontmost of process = false`
  throughout the client run. No pointer movement exists in the tool
  path — there is no input synthesis anywhere in the prototype.

## Snapshot freshness matrix

| window state | DOM bracket | PNG shows | verdict |
|---|---|---|---|
| frontmost (key) | 162 / 162 | 162 | fresh |
| unfocused, visible | 136 / 137 | 136 | fresh |
| fully occluded (Terminal window covering it) | 186 / 186 | 186 | fresh |
| minimized to Dock | 206 / 206 | 206 | fresh |
| restored after minimize | 237 / 237 | 237 | fresh |
| another Space | — | — | not probed (not scriptable without Mission Control UI scripting) |

PNGs in `shots/`. No permission prompt, no private API, no entitlement —
`takeSnapshot` on a debug binary worked in every state.

## Page-side timing (WKWebView, measured in-page)

- `setInterval(1000)` fires at ~0.5 Hz in **every** window state,
  including frontmost-key (5 ticks per 10 s). A 100 ms interval fired 9
  times in 3031 ms unfocused. Timer coalescing is state-independent.
- `requestAnimationFrame` runs at ~60 fps (182 frames/3 s) only while the
  window is key; with the window merely unfocused-but-visible it does not
  fire at all (0 frames/3 s). `document.visibilityState` stays `visible`
  and `document.hasFocus()` false in that state.
- Consequence for the freshness trick: the counter is not a wall clock.
  Bracketing with `evaluate` is what makes freshness decidable; absolute
  tick rates cannot be assumed. Consequence for contract 022: agents
  cannot rely on rAF-driven visuals advancing while the app is unfocused;
  snapshots reflect DOM state faithfully regardless.

## Crate versions (Cargo.lock)

tauri 2.10.3 (tauri-runtime / tauri-runtime-wry pinned 2.10.1 — 2.11.x
breaks tauri 2.10.3 compilation), wry 0.54.4, rmcp 3.1.3, axum 0.8.9,
tokio 1.53.1, objc2-web-kit 0.3.2, objc2-app-kit 0.3.2, block2 0.6.2,
base64 0.22.1.
