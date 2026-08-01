# 079 Retained Notification Ledger And Operation Observation

Status: complete
Owner: Tom
Roadmap: g01.012 batch 3
Governing refs: contracts 001, 012, 015, and 016; research memo 016
Depends on: Card 078
Auto-start next card: no

## Objective

Implement the pure finite notification ledger. Keep record state independent
from transient presentation and add an optional failure-isolated operation
outcome projector.

## Scope

- `longhorn-notifications` crate
- notification, source, replacement-key, and action-reference identity
- bounded severity, title, summary, cause, actions, and ordering
- add and explicit replace-by-key
- distinct unseen, seen, dismissed, clear, and prune outcomes
- count and encoded-weight retention
- newest-first bounded projections and unseen count
- optional operation terminal-transition projector
- Loophole render and non-operation reliability fixtures

## Out Of Scope

- TypeScript, Tauri, Svelte, Poodle, or toast timers
- native OS notification delivery
- operation authority mutation
- action execution or authorization
- product logs, recovery evidence, and artifacts
- donor repository writes

## Steps

1. Add the pure crate and bounded identity/revision types.
2. Define notification record, severity, cause, and semantic actions.
3. Implement add and explicit replace-by-key transitions.
4. Separate mark-seen, dismiss, bounded clear, and retention prune.
5. Implement authoritative pages and exact unseen count.
6. Add optional idempotent operation-outcome projection.
7. Prove notification publication failure cannot change operation outcome.
8. Freeze Loophole render and reliability record fixtures.
9. Audit operation-root independence and product-data leakage.

## Acceptance Criteria

- operation and non-operation records use one ledger
- notification root works without operation dependency
- title text never drives deduplication
- seen, dismissed, clear, and prune are distinct and receipted
- unseen count is derived from explicit state, not retained count
- retention is finite and reports every removal
- action references are bounded data, not executable closures
- operation projection is optional, idempotent, and failure-isolated
- pure graph imports no bridge, Tauri, async runtime, Svelte, or Poodle

## Evidence Required

- record transition and exact-state rejection matrix
- replace-key and deduplication-token fixtures
- seen/dismiss/clear/prune fixtures
- count/weight retention and overflow fixtures
- render-terminal and reliability-event fixtures
- operation publication-failure isolation fixture
- dependency and public-API audit
- focused Rust, clippy, docs, formatting, and Effigy checks

## Stop Conditions

- retained notification truth requires renderer toast state
- operation state must be mutated to publish a record
- action execution must enter the pure ledger
- safe metadata bounds cannot be enforced without product payloads
- notification root cannot remain independent of operation authority

## Next Task

Card 080 is ready after the ledger passes. Generate checked clients and add
Svelte/Poodle panel, toast, and semantic-action projections.

## Completion Evidence

- `longhorn-notifications` owns bounded record identity, metadata, explicit
  read state, finite retention, and authoritative newest-first projections.
- Add never deduplicates by title. Replacement requires a unique source/key;
  idempotent publication requires a separate durable producer token.
- Seen, dismissed, cleared, and pruned outcomes are distinct and receipted.
  Count and encoded-weight pressure reports every oldest-first removal.
- Protected records and newly admitted records are never silently dropped.
  Unsatisfiable limits, stale state, and numeric overflow leave exact state.
- Loophole render and non-operation reliability fixtures share one record
  shape without importing product payloads.
- The root dependency graph contains only `longhorn-core`. Optional operation
  observation accepts immutable committed terminal evidence; a forced
  notification failure cannot change the operation catalogue.
