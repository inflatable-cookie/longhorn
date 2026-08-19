# Agent Control Semantic Surface

Date: 2026-08-19
Scope: g02.032 (Cards 232-234)

## What happened

The last agent-control lane delivered the in-page semantic shim, wired the
remaining tools through it, and proved a packaged app can be driven
unfocused. Refs live as `data-longhorn-agent-ref` attributes on the
elements they name. Truncation is a sentinel child with role `truncated`
(the core vocabulary has no truncation field). Synthetic input is the
documented untrusted DOM sequence. `wait_for` pacing stays host-side.

`subscriptions/listen` works on rmcp 3.1.3 for revision 2026-07-28.
Custom notifications, logging, and progress are rejected by the typed
sink. Console, page-error, and navigation events therefore ride as three
MCP resources; subscribers opt in by URI and receive
`notifications/resources/updated`. The resource body carries the bounded
ring and the drop counter. Contract 022 records that mapping.

## Evidence

- Shim fixtures in `packages/longhorn/tests/agent-control/` (tree, refs,
  input sequences, wait predicates, standalone IIFE).
- Plugin marshalling tests in `crates/longhorn-tauri-agent-control/src/shim.rs`
  plus mount fixtures (tools wired, resources listed, listen acknowledged).
- Core conformance: event resources listed and readable.
- Release-absence scan extended with shim markers, both directions.
- Packaged driver `examples/agent-control-proof/e2e.ts`. Receipt
  `examples/agent-control-proof/evidence/2026-08-19T20-34-25-e2e/`
  (`appHeldFocus: false`, shared refs, both listen streams, discovery
  removed on quit).

## Worker loop

Handoff
`docs/handoffs/20260819-194148-agent-control-semantic-surface-worker.md`.
Launcher worktree `/Users/tom/.t3/worktrees/longhorn/t3code-672f3dc8` on
`t3code/follow-agent-control-handoff` (clean, dedicated, non-`main`; used
as supplied). `origin/main` at `bb22d6fb`; `bac9a646` is an ancestor.
