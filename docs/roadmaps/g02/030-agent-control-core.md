# g02.030 Agent Control Core

Status: planned — blocked on g02.029 evidence and contract 022 promotion
Owner: Longhorn maintainers
Created: 2026-08-19
Governing refs: contract 022 (promotion pending); contracts 001, 006, 012
Depends on: g02.029 complete; contract 022 active

## Outcome

`longhorn-agent-control` exists: the host-agnostic core of the in-app agent
control surface — tool definitions, discovery-file lifecycle, per-instance
token auth, and the native-surface provider seam — with stateless
conformance proved without any host.

## Generation Runway

Cards compile to ready after promotion; numbers reserved:

- [ ] Card 228 — core crate: tool schema, discovery file (app id, pid,
      port, token, schema version), token generation, provider seam.
- [ ] Card 229 — stateless server assembly over rmcp: no minted session
      ids, Origin rejection, bad-token rejection, discovery lifecycle
      fixtures including stale-pid detection.

## Acceptance

- No session id ever appears in a response.
- Discovery files are created, enumerable, stale-detectable, and removed on
  clean exit.
- Origin and token failures reject before any tool executes.
- The crate compiles with no host dependency; contract 012 gates pass.
