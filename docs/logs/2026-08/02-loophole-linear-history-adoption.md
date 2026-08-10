# Loophole Linear History Adoption

Date: 2026-08-02
Roadmap: g01.015
Card: 111
State: complete

## Changed

- Replaced Pulse's generic stack with a facade over Longhorn's structural
  `LinearHistory<PulseHistoryMutation>` authority.
- Registered Pulse-owned codec and policy families covering all 83 mutation
  variants, inverse, no-op, coalescing, explicit groups, 750 ms automatic
  groups, and the retained limit of 100.
- Added a versioned canonical Longhorn envelope beside a complete legacy
  rollback projection. Canonical disagreement, corruption, and future formats
  fail visibly; legacy-only snapshots import directly without product replay.
- Routed undo, redo, and checkout through revision-bound plan/apply/commit.
  Pulse applies the complete product batch transactionally and restores exact
  product and structural state on failure.
- Kept project versions, mutation application, autosave, checkpoint recovery,
  and the project-adjacent JSONL journal under Pulse authority. Committed
  structural movement emits exactly one journal record per moved step.
- Added a caller-authorized Aura Tauri history host and renderer session with
  authoritative paged past/current/future metadata and the shared Poodle panel.
- Retained the old eight-entry Pulse session projection only as a compatibility
  snapshot and external-mutation invalidation signal. It is not renderer
  history authority.
- Kept branch packages and fork-tree semantics absent.

## Authority Boundary

Longhorn owns linear identity, ordering, revision checks, grouping structure,
retention, navigation planning, persistence envelope, transition receipts,
metadata paging, and shared clients. Pulse owns every payload field and
meaning, inverse/coalesce/no-op policy, runtime apply transaction, labels,
project lineage, journal, autosave, and recovery decisions.

## Evidence

- the migration receipt (retired 2026-08-10; in git history)
- verifier removed 2026-08-10 — Longhorn no longer keeps consumer-aware proofs; the recorded fixture is the retained evidence
- Coverage derives 83 enum variants and proves inverse and apply parity for
  every variant.
- Persistence tests cover complete applied/future ordering, canonical and
  legacy round trips, corruption, disagreement, and legacy-only import.
- Navigation tests cover stale plans and multi-entry partial failure with exact
  product and structural rollback.
- Host and renderer tests cover checked commit-once navigation, paging,
  past/current/future projection, and filtering.

## Validation

- Card 111 verifier: `pass_with_product_owned_payload_policy`.
- Longhorn history protocol tests: 4 passed.
- Pulse history: 30 passed; Pulse full suite: 384 passed.
- Pulse persistence: 69 passed.
- Aura renderer: 97 files and 980 tests passed.
- Full Loophole validation passed: Aura, Echo 414, Pulse 384, and Spark 11.
- Aura Svelte check: 0 errors; one existing tsconfig warning.

## Next

Execute Card 112. Prove full migration conformance, remove any remaining active
generic duplicates, record retained adapters, and close g01.015.
