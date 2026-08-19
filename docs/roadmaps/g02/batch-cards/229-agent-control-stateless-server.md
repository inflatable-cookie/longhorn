# 229 Agent Control Stateless Server

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.030
Governing refs: contract 022; memo 024; contracts 001, 012
Depends on: Card 228
Auto-start next card: no — g02.031 is a separate dispatch

## Objective

`longhorn-agent-control` serves its tool surface as a stateless MCP
streamable-HTTP server, proved without any host: an axum router a host
later mounts, with auth, Origin policy, and discovery wired.

## Scope

- rmcp `StreamableHttpService` assembly with sessions off (Card 227's
  configuration: `legacy_session_mode: false`), tools dispatched to a
  handler trait the host implements. Pin rmcp at the 3.x line proved by
  the spike; record the exact version.
- Bearer-token middleware (constant-time compare against the instance
  token) and `Origin` validation rejecting browser-originated requests,
  both running before any tool executes.
- Server binds 127.0.0.1 only; port 0 supported; the bound port feeds the
  Card 228 discovery file, whose lifetime is tied to the server's.
- Conformance fixtures over an in-process client (no network flakiness):
  - no `Mcp-Session-Id` is ever minted or echoed;
  - GET and DELETE answer 405;
  - missing/wrong token → rejected before dispatch;
  - present-and-invalid `Origin` → 403;
  - discovery file appears on serve, disappears on shutdown, and a dead
    pid is detectable by an enumerator;
  - two clients interleaving calls see no cross-talk.

The dev-only gating lives with the host (g02.031); this crate is inert
until composed, per the g02 consumer guardrails.

## Acceptance Criteria

- [ ] all conformance fixtures above pass in CI-shaped `effigy qa`
- [ ] the auth and Origin path is unit-proved to run before tool dispatch
- [ ] no session id appears in any response across the fixture suite
- [ ] exact rmcp version and its negotiated-revision behavior recorded in
      the card closeout (memo 024 records 3.1.3 defaults to 2025-11-25
      while supporting 2026-07-28 — confirm or correct for the pinned
      version)
- [ ] `effigy qa` passes

## Validation

`effigy qa`; `effigy doctor`.

## Stop Conditions

- rmcp's server surface cannot express the auth-before-dispatch ordering —
  stop and report before wrapping it in middleware hacks;
- statelessness would require holding any per-client state to make a tool
  work — that contradicts contract 022 and needs the orchestrator.

## Continuation

g02.030 closes with this card. g02.031 (Tauri host, capture, release
absence) compiles next; its cards are reserved as 230-231.
