# 227 Agent Control Spike

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.029
Governing refs: research memo 024; contract 022 (draft — evidence target,
not execution authority); contracts 001, 020
Depends on: none
Auto-start next card: no — g02.030 gates on promotion

## Objective

Answer memo 024's two runtime gaps with a standalone prototype, so
contract 022 can be promoted from facts:

1. Does a stateless MCP server (rmcp, streamable HTTP) mount and serve
   inside a running Tauri app, and which protocol revision does a current
   agent client actually negotiate against it?
2. Is `WKWebView.takeSnapshot` fresh for an unfocused, fully occluded
   window — and what actually happens when the window is minimized or on
   another Space?

## Scope

Everything lives under `prototypes/agent-control/`, outside the workspace,
like the other prototype directories. No shared crate or package changes.
The prototype is throwaway evidence, not donor code.

Build the smallest Tauri app that:

- renders a webview page with a visibly changing element (e.g. a counter
  ticking every second) so snapshot freshness is decidable from the image;
- mounts rmcp's `StreamableHttpService` on an axum router inside the app
  process, bound to 127.0.0.1 with a random port, stateless configuration;
- exposes two MCP tools: `evaluate` (run JS in the webview, return the
  result) and `screenshot` (WKWebView `takeSnapshot` via
  `with_webview`, returned base64);
- writes a discovery line (port) to stdout or a temp file so the client
  can connect.

Then run the probes and record results:

- connect a real agent client (Claude Code MCP config is sufficient);
  record the negotiated protocol revision and whether any session id
  appears on the wire;
- capture screenshots with the window frontmost, fully occluded by
  another window, minimized, and on another Space where practical; for
  each, record whether the image shows the current counter value or a
  stale one;
- call `evaluate` while the app is unfocused; confirm the OS pointer and
  focus never move.

## Acceptance Criteria

- [ ] prototype builds and runs from `prototypes/agent-control/`
- [ ] a real MCP client lists and calls both tools over streamable HTTP
- [ ] negotiated protocol revision and session behavior recorded
- [ ] occluded-window snapshot freshness recorded with the method stated
- [ ] minimized / other-Space behavior recorded, even if the answer is
      "stale" or "fails"
- [ ] memo 024's Gaps section updated: each closed gap moves to Findings
      with its evidence; anything contradicting contract 022's draft is
      recorded in the memo, with the contract left untouched
- [ ] card closeout notes the actual crate versions used (tauri, rmcp,
      objc2-web-kit)

## Validation

`cargo build` and a manual run of the prototype; `effigy doctor` for repo
hygiene. Full `effigy qa` is not required — the workspace is untouched —
but run it if any shared file changes for any reason, and stop to report
why a shared file needed to change.

## Stop Conditions

- rmcp cannot mount statelessly inside the app process, or a current
  client cannot complete a tool call against it;
- capture requires private API, a permission prompt, or returns stale
  images for a merely occluded (not minimized) window;
- the spike starts wanting shared-crate changes — that is g02.030's work
  and needs promotion first.

On any of these, record what was observed in memo 024 and stop; a negative
result is a successful spike.

## Continuation

Evidence lands in memo 024 and this card. Promotion of memo 024 and
contract 022, and compilation of g02.030-032 to ready, is orchestrator
work, not this card's.
