# g02.044 Agent-Control Stdio Carrier

Owner: Tom
Created: 2026-09-22
Governing refs: contract 022 (`Availability And Security`, `Tool Surface`, `Discovery`)
Depends on: no other task
UI classification: none

## Outcome

A Longhorn-owned, opt-in stdio carrier — `longhorn-agent-control-client` — that a
harness can spawn as an MCP server and that **fronts** the live Contract 022
instance over stdio: one server, one registered catalogue, one policy, one
dispatch path. It reimplements nothing and owns no catalogue.

## Context

Contract 022's server is streamable HTTP on loopback. Some harnesses (Claude
Code, the Claude Agent SDK sidecar, Grok Build ACP, Codex, and others) may not
accept a consumer-supplied streamable-HTTP server entry, so a stdio carrier is
required for them. Swallowtail raised the requirement on 2026-09-22 and is
publishing per-route evidence for which harnesses actually need it. The carrier
is Longhorn-owned by requirement: a Swallowtail-side bridge would itself be a
second production MCP server, which the 2026-09-22 direction withdrew. This
promotes the "Agent-control stdio proxy client" candidate from the Tier A
runway.

## Per-route evidence (Swallowtail g06.014, `7fe38435`)

Swallowtail published the measured route list
(`docs/research/336-consumer-supplied-http-mcp-acceptance-per-route.md`):

- **`direct-http`: none.** No route accepts a consumer-supplied streamable-HTTP
  MCP entry, so the carrier is the only path to the production MCP for every
  Swallowtail route today. Ungating it was right, not optimistic.
- **`carrier-required`:** `claude-agent.sdk` (consumer-declared stdio servers
  only; SSE/HTTP is not representable on that seam) and `grok-build.catalogue` +
  `grok-build.acp` (the ACP `mcpServers` entry is proven only for a
  Swallowtail-owned stdio courier, only at exact `1.0.4`/`1.0.5`). These two are
  the honest acceptance set.
- **Not carrier dependents:** `codex.app-server` is a provider limitation — no
  typed per-session client-declared MCP surface exists across the qualified
  range — and the six `producer-gap` ACP rows (`cline.acp`, `copilot-cli.acp`,
  `gemini-cli.acp`/`.headless`, `goose.acp`, `kiro.acp`, `deepagents.acp`) are
  **not** sized for until a promoted Swallowtail `g06.005` route plus its live
  gate moves one into `carrier-required`.
- **`subscriptions/listen` and the `longhorn://agent-control/...` resources:** no
  route needs them today. Record them typed `Unsupported` with an explicit
  reopen condition.
- Swallowtail's own stdio couriers are non-production development surfaces: they
  prove stdio *admission*, never harness HTTP acceptance, and must not be cited
  as evidence that a harness accepts anything.

## Ready-State Rubric

- [x] The operator accepted the requirement in principle (Swallowtail direction,
      2026-09-22).
- [x] Ownership is Longhorn — not Swallowtail, and not a general-purpose adapter.
- [x] The operator directed the carrier into the `0.2.0` release (2026-09-22), so
      dispatch is not gated on the per-route evidence; validation uses the
      harnesses that evidence names as it lands, and at minimum one stdio
      harness.
- [x] UI classification is none.

## Work

1. Add the `longhorn-agent-control-client` binary to a Longhorn crate. The
   distribution decision — crate bin versus npm — is inside this task.
2. Discover the live instance through `longhorn-agent-control`'s discovery
   directory and its per-instance bearer.
3. Proxy stdio MCP ↔ the instance's streamable-HTTP MCP endpoint. Preserve
   contract 022 semantics: stateless self-contained requests, no minted or
   echoed session id, the same tool list and result/error shapes, and
   cancellation on stream close.
4. Take an explicit position on `subscriptions/listen` events and the
   `longhorn://agent-control/...` resources over stdio: adapt them, or
   deliberately do not claim them and answer typed `Unsupported`.
5. Enforce the per-instance bearer and loopback origin through the carrier: no
   widened exposure, and no route to a second catalogue.
6. Opt-in character consistent with `agent-control`: a build without the carrier
   contains none of it.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| One server, one catalogue | the carrier reaches the discovered instance and holds no catalogue of its own |
| Semantics preserved | a harness driving the carrier sees the same tools, results, errors, and cancellation as the HTTP client |
| No widened exposure | the carrier binds only the discovered loopback instance with its bearer; it exposes no new listener |
| Opt-in | a build without the carrier contains none of it |
| Measured need | validated against `claude-agent.sdk` and `grok-build.acp` (version-scoped `1.0.4`/`1.0.5`); no other route is a carrier dependent |

Validation uses the focused agent-control selectors plus an end-to-end run
against the harnesses the evidence names, then `effigy qa`. An independent
exact-head review must inspect the carrier-to-server path, not prose.

## Stop conditions

Stop if the carrier would need its own catalogue, registry, lease, or listener,
or if a route can only be served by a Swallowtail-owned production listener.
Escalate to Chatterbox.

## Evidence

On completion, record: the binary, the discovery and bearer path, the
listen/resources position, and the harnesses validated against.
