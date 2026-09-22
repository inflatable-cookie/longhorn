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
| Measured need | validated against at least one stdio harness end-to-end, and against the routes the per-route evidence names as it lands |

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

Completion record (2026-09-22):

- Binary: `longhorn-agent-control-client`, a `[[bin]]` in
  `crates/longhorn-agent-control` behind the non-default `client`
  feature. Distribution decision: crate bin, not npm — the carrier is a
  host-local loopback companion spawned by the harness, not a JS
  package. Opt-in proof: the default graph has no `reqwest` and no
  binary target (`cargo tree -p longhorn-agent-control -e normal` shows
  neither); the feature adds HTTP-only `reqwest` (no TLS back end, so
  `https` URLs cannot be built) plus `tokio/io-std` and
  `tokio/process` for stdio and the e2e child spawn.
- Discovery and bearer path: `--discovery-dir` wins, else `--state-root`
  as an explicit contract-004 override, else the platform-native
  directory resolved from process-environment facts (same XDG/`Library`
  rules the Tauri host injects). Exactly one live instance is selected
  (`--app-id` disambiguates); zero or many exits 2 without spawning.
  The bearer travels only in the `Authorization` header to
  `http://127.0.0.1:<port>/mcp`; no `Origin` header is sent (absent
  passes the guard) and no listener is bound.
- Strict-path translation: the carrier synthesizes the SEP-2243 headers
  (`Mcp-Method`, `Mcp-Name`, `Mcp-Param-*` from cached `tools/list`
  schemas, `MCP-Protocol-Version` from the negotiated version) and
  additively merges the `_meta` envelope a plain stdio harness omits —
  never rewriting a harness-declared `_meta`. Below the negotiated
  standard version the body stays legacy-bare. Plain-harness requests
  would otherwise all fail the server's `_meta` requirement.
- Listen/resources position: adapted by pass-through. `resources/list`,
  `resources/read`, `resources/subscribe`, and `subscriptions/listen`
  travel the same POST path; a listen SSE stream stays mapped onto its
  pending stdio request and `notifications/resources/updated` arrives on
  it. `notifications/cancelled` aborts the in-flight POST (stateless
  HTTP holds nothing server-side to cancel); stdin EOF aborts all and
  exits 0.
- Harnesses validated: no vendor harness (Claude Code, Codex, ACP) is
  runnable in this environment, so validation is the `tests/carrier.rs`
  end-to-end run, which spawns the built binary over piped stdio and
  speaks newline-delimited JSON-RPC exactly as a stdio harness would:
  `initialize`/`tools/list`/`tools/call` replies asserted byte-equal to
  the same exchanges made directly over HTTP, plus listen delivery,
  cancellation survival, and exit-2 selection errors. Per-route vendor
  evidence from Swallowtail had not landed at dispatch; re-run the e2e
  against the routes it names when it does.
