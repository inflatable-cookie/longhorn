# 228 Agent Control Core Crate

Status: done
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

- [x] crate compiles host-free; no tauri, wry, or objc2 dependency
- [x] tool vocabulary covers the contract 022 surface, including typed
      untrusted-event limits where they belong (drag has no OS-level mode)
- [x] discovery lifecycle proved by fixtures: create, enumerate, stale-pid
      detection, idempotent remove
- [x] token never appears in logs or Debug formatting, proved by a fixture
- [x] no `wait_for` variant can express a time-only or rAF wait
- [x] `effigy qa` passes with the new crate in the graph

## Closeout

Status: done 2026-08-19. Worker branch `worker/230-agent-control-core`,
worktree `~/Dev/worktrees/longhorn-230` (manual fallback container; the
planning checkout was on `main`).

The crate landed as `tools` (vocabulary), `discovery`, `token`, and
`provider` modules with no server and no host dependency. Notable shape
decisions:

- Discovery path resolution runs through contract 004's
  `resolve_storage_layout` with the fixed `longhorn` identity and the
  `platform-native-v1` profile, so the per-platform shapes are the
  profile's own state-root rules and tests inject roots as overrides —
  no hand-rolled dirs lookup, no filesystem or environment access in
  resolution.
- Stale detection ships a real probe: `process_alive` over `sysinfo`
  (std has no safe liveness API and the workspace forbids `unsafe`;
  hand-rolled per-platform code is what the host-free criterion forbids
  in this crate). **Reviewer note:** sysinfo transitively pulls
  `objc2-io-kit`/`objc2-core-foundation` on Apple targets. The crate's
  own manifest carries no tauri/wry/objc2 and no platform-specific code;
  the acceptance line reads as host-coupling, which this is not — but
  the transitive pair exists in the macOS graph and is flagged rather
  than hidden.
- The token is 32 CSPRNG bytes base64url (43 chars), `SecretString`-held,
  constant-time verify, redacted `Debug`; plaintext only in the discovery
  file, which exists to carry it (owner-only file and directory modes).
- `wait_for` admits exactly four DOM-relative predicates; a fixture
  proves the wire rejects duration/animation-shaped variants. Drag
  carries no mode field and `deny_unknown_fields` keeps one off the wire.
- The provider seam stayed at two methods over the shared vocabulary;
  it did not want to grow.

Fixtures: 19 across `tests/discovery.rs` and `tests/token.rs` plus module
unit tests. Validation: `effigy qa` exit 0 with the crate in the graph
(run at the runway's end, after Card 229).

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
