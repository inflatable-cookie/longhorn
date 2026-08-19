# 228 Agent Control Core Crate

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.030
Governing refs: contract 022; memo 024; contracts 001, 006, 012
Depends on: Card 227 (evidence base)
Auto-start next card: yes — Card 229 in the same lane

## Objective

`longhorn-agent-control` exists as a workspace crate: the host-agnostic
vocabulary and mechanics of the control surface, with no server and no
host dependency.

## Scope

- **Tool vocabulary as types.** Requests, results, and errors for the
  contract 022 tool surface: `snapshot`, `click`, `type`, `press`,
  `scroll`, `drag`, `evaluate`, `wait_for`, `screenshot`, `command`, and
  window operations. Element refs are opaque strings resolved by the edge
  that stamped them; the core never holds a ref table (contract 022's
  stateless posture). `wait_for` predicates are expressed against the
  semantic tree or page state — no duration-only or animation-frame
  variant exists, per the contract's DOM-relative rule.
- **Discovery file.** Serde model and lifecycle (create, enumerate,
  detect-stale-by-dead-pid, remove) for
  `<state-dir>/longhorn/agent-control/<app-id>-<pid>.json` carrying app
  id, pid, port, token, and schema version. Path resolution follows the
  existing storage-profile conventions (contract 004's roots), not a
  hand-rolled dirs lookup.
- **Token.** Per-instance bearer token generation and constant-time
  verification. The token is a credential: never logged, never in Debug
  output.
- **Provider seam.** The trait a native (non-webview) surface implements
  later to contribute snapshot and action handling. Sealed enough that
  absence is fine (contract 020: a GPUI host composing nothing is not a
  gap); no provider ships here.

Workspace admission per contract 012: crate added to the graph, deny/MSRV
gates passing, no consumer composes it yet.

## Acceptance Criteria

- [ ] crate compiles host-free; no tauri, wry, or objc2 dependency
- [ ] tool vocabulary covers the contract 022 surface, including typed
      untrusted-event limits where they belong (drag has no OS-level mode)
- [ ] discovery lifecycle proved by fixtures: create, enumerate, stale-pid
      detection, idempotent remove
- [ ] token never appears in logs or Debug formatting, proved by a fixture
- [ ] no `wait_for` variant can express a time-only or rAF wait
- [ ] `effigy qa` passes with the new crate in the graph

## Validation

`effigy qa` (shared workspace changes). `effigy doctor` for orientation.

## Stop Conditions

- the tool vocabulary needs a concept contract 022 does not admit (native
  chrome interaction, trusted input, remote access) — stop, that is a
  contract question;
- discovery cannot follow contract 004 root conventions without extending
  them — stop and report rather than inventing a parallel root.

## Continuation

Card 229 assembles the stateless server over this vocabulary in the same
worker lane.
