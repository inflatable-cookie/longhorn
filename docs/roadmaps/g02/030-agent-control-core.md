# g02.030 Agent Control Core

Status: ready
Owner: Longhorn maintainers
Created: 2026-08-19
Governing refs: contract 022; memo 024; contracts 001, 006, 012
Depends on: g02.029 complete and contract 022 active — both true 2026-08-19

## Outcome

`longhorn-agent-control` exists: the host-agnostic core of the in-app agent
control surface — tool definitions, discovery-file lifecycle, per-instance
token auth, and the native-surface provider seam — with stateless
conformance proved without any host.

## Generation Runway

- [ ] [Card 228](batch-cards/228-agent-control-core-crate.md) — core
      crate: tool schema, discovery file (app id, pid, port, token, schema
      version), token generation, provider seam.
- [ ] [Card 229](batch-cards/229-agent-control-stateless-server.md) —
      stateless server assembly over rmcp: no minted session ids, Origin
      rejection, bad-token rejection, discovery lifecycle fixtures
      including stale-pid detection.

## Acceptance

- No session id ever appears in a response.
- Discovery files are created, enumerable, stale-detectable, and removed on
  clean exit.
- Origin and token failures reject before any tool executes.
- The crate compiles with no host dependency; contract 012 gates pass.
