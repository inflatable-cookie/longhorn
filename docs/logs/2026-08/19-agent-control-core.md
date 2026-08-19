# Agent Control Core

Date: 2026-08-19
Scope: g02.030 (Cards 228-229, PR 3); contract 022 wording alignment

## What happened

The second worker lane of the agent-control runway delivered
`longhorn-agent-control`: the host-agnostic core of the contract 022
surface. Tool vocabulary as types (refs opaque, `wait_for` admitting only
DOM-relative predicates with duration/animation variants unrepresentable
and wire-rejected), discovery-file lifecycle through contract 004's
resolver with owner-only modes, a `SecretString`-held CSPRNG token with
constant-time verify, a two-method provider seam, and the stateless rmcp
server assembly with auth and `Origin` enforcement layered structurally
ahead of dispatch.

## Evidence

26 tests: 6 conformance fixtures (no session id minted or echoed,
GET/DELETE 405, 401/403 at zero dispatches on a counting stub, discovery
lifetime tied to a real loopback server, 16-call two-client interleave
with journal-exact correspondence), 9 discovery, 3 token, 8 unit.
`effigy qa` exit 0 in the worker worktree; the orchestrator independently
re-ran the crate suites, clippy, and fmt at the PR head before verdict.
rmcp 3.1.3 confirmed memo 024's revision behavior; no correction.

## Deviations accepted in review

- `sysinfo` pulls transitive `objc2-*` on Apple targets for pid liveness;
  judged not host-coupling, flagged in the Card 228 closeout.
- The `Origin` guard admits loopback browser origins (token still
  required) rather than rejecting every browser origin. Judged the better
  rule — DNS rebinding still presents the attacker's origin; localhost
  tooling like MCP Inspector keeps working — and contract 022's wording
  was aligned to it in this closeout commit.

## Worker loop

Handoff `docs/handoffs/20260819-154752-agent-control-core-worker.md`,
manual-fallback worktree `~/Dev/worktrees/longhorn-230`, PR 3 reviewed and
merged on operator authorisation; worktree and branch removed after merge.
Neither card stop condition fired. g02.031 (Cards 230-231, Tauri host and
capture) compiled to ready — the first lane needing a macOS host and a
packaged proof app.
