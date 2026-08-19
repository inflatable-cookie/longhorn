# 229 Agent Control Stateless Server

Status: done
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

- [x] all conformance fixtures above pass in CI-shaped `effigy qa`
- [x] the auth and Origin path is unit-proved to run before tool dispatch
- [x] no session id appears in any response across the fixture suite
- [x] exact rmcp version and its negotiated-revision behavior recorded in
      the card closeout (memo 024 records 3.1.3 defaults to 2025-11-25
      while supporting 2026-07-28 — confirm or correct for the pinned
      version)
- [x] `effigy qa` passes

## Closeout

Status: done 2026-08-19. Same branch and worktree as Card 228.

**rmcp version and revision behavior:** pinned `rmcp = "3.1.3"` (the
newest published on 2026-08-19; the spike's line), resolved exactly 3.1.3
in the workspace lock. Confirmed against the vendored source, memo 024
stands uncorrected: `ProtocolVersion::LATEST` is `V_2025_11_25`;
`V_2026_07_28` is a supported constant; `server/discover` lists 2024-11-05
through 2026-07-28 (visible in the Card 227 wire capture). With
`legacy_session_mode: false` and no event store, the service is POST-only
and mints no session ids; GET/DELETE fall through to 405 with
`Allow: POST`.

**Auth-before-dispatch:** expressed as an axum layer outside the rmcp
service — a rejected request (401 bad/missing bearer, constant-time
compared; 403 non-loopback `Origin`) never enters the MCP service, so the
ordering is construction, not middleware convention. The stop condition
did not trigger: no rmcp internals were touched. rmcp's own default
loopback `Host` validation stays on underneath as a second DNS-rebinding
layer. The wire args are flat schemars structs in `server/args.rs` that
validate into the Card 228 vocabulary, keeping the vocabulary free of
schema dependencies and `longhorn-core` free of schemars.

**Fixtures** (`tests/conformance.rs`, 6): no `Mcp-Session-Id` minted or
echoed (including a client-supplied bogus one); GET/DELETE → 405;
missing/wrong token → 401 with the stub's invocation counter at zero;
browser `Origin` (including `null` and a lookalike host) → 403 at zero
dispatches, loopback and absent origins admitted; discovery file appears
on serve with the resolved ephemeral port, answers a real-loopback
tools/list, and disappears on shutdown; two clients interleaving 16
concurrent calls see exact echo correspondence and a complete journal.

Resolved dependency versions (workspace lock): rmcp 3.1.3, axum 0.8.9,
tokio 1.53.1, schemars 1.2.2 (shared with rmcp's `1.0` requirement),
sysinfo 0.39.6, tower 0.5.3 and http-body-util 0.1.4 (dev-only).

Validation: `effigy qa` exit 0. Worktree bootstrap needed the 2026-08-16
papercut path: `bun install`, then `effigy deps link bun` against the
sibling Poodle checkout after removing the registry symlinks it refuses
to replace, and `POODLE_REPO` set for the proofs.

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
