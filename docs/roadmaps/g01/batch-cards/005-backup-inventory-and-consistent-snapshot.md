# 005 Backup Inventory And Consistent Snapshot

Status: complete
Owner: Tom
Completed: 2026-07-28
Roadmap: g01.002 batch 3
Governing refs: contracts 001, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Add registry-driven backup policy, explicit scope, strict manifest types, and a
bounded immutable snapshot of ordinary published configuration captured under
the existing store-wide coordination guard.

## Scope

- included, excluded-with-reason, and custom-adapter registry policy
- explicit operation scope and stable domain-id ordering
- backup identity, kind, app/producer metadata, and consistency-group model
- strict version-1 manifest Rust types without ZIP encoding
- present, absent, and source-preserved domain evidence
- exact source schema, bytes, length, and SHA-256
- bounded per-domain and total snapshot limits
- one under-lock inventory and capture cut for ordinary file-backed domains
- private immutable in-memory snapshot after guard release
- typed unreadable, unavailable, limit, coordination, and policy failures
- machine-readable capture receipt

## Public Behavior

Backup policy is separate from `ConfigDomain`. Product schemas do not need
backup code. One catalogue associates registered domain ids with include,
exclude, or custom policy and rejects missing or duplicate decisions for a
selected scope.

Ordinary capture:

1. acquire the store coordinator with a finite caller policy
2. enumerate selected domains in stable id order
3. read exact registered source bytes or absence while holding the guard
4. classify valid, older, future, corrupt-readable, absent, or unavailable
5. reject unreadable required sources and configured byte-limit overflow
6. compute length and SHA-256 and freeze the bounded snapshot
7. release the guard and return manifest inputs plus receipt

Missing files stay absent. Compiled defaults are not encoded as persisted
payloads. Future and corrupt-readable bytes may be retained only as
`source-preserved`; that state is explicit and not ordinary-restorable.

Capture covers persisted state only. It neither observes nor flushes
`DebouncedMutation` lanes.

Exact Rust names may vary. Stable ordering, evidence shape, size enforcement,
and guard lifetime may not.

## Out Of Scope

- ZIP serialization, compression, file publication, export, or retention
- restore inspection, planning, migration staging, journal, or rollback
- age encryption or key providers
- custom adapter execution
- pre-migration destructive rewrite
- Tauri, TypeScript, Svelte, Poodle, or consumer migration

## Steps

1. Characterize a SHA-256 dependency under Rust 1.85.
2. Add backup policy, scope, ids, kinds, source states, limits, strict manifest
   model, and receipts.
3. Expose stable registry inventory without exposing mutable registry
   internals.
4. Add an internal capture path that receives the existing non-reentrant
   coordination guard.
5. Capture exact present bytes and absence in stable order; retain source
   schema evidence for valid, migrated, future, and corrupt-readable files.
6. Enforce checked per-domain and total byte limits before snapshot commit.
7. Add concurrency, recovery-state, limit, and pending-debounce fixtures.
8. Run the complete card validation. Stop before archive encoding.

## Acceptance Criteria

- every selected registered domain has one explicit policy decision
- secret, cache, runtime, and log policies default to excluded
- selected ids and manifest entries are stable by domain id
- concurrent cooperating mutation cannot interleave with the captured cut
- present payload bytes, length, source schema, and SHA-256 match disk
- absence does not materialize a default
- older valid source is captured without destructive migration
- future and corrupt-readable source is preserved and marked non-restorable
- unreadable required source fails the whole capture
- per-domain, total, and arithmetic-overflow limits fail before snapshot
  return
- pending debounced intent is absent unless the host flushed it first
- guard is released before later archive work could begin
- package graph remains free of ZIP, age, Tauri, async runtime, Svelte, and
  Poodle

## Evidence Required

- policy completeness and duplicate fixtures
- stable-order manifest snapshot
- absent, current, migrated, future, corrupt-readable, and unreadable fixtures
- exact SHA-256 and byte-length vectors
- per-domain, total, and checked-overflow limit tests
- helper-process mutation blocked until capture release
- pending debounce versus forced-flushed capture fixture
- Rust 1.85 workspace check
- `effigy doctor`
- `effigy test --plan`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`

## Stop Conditions

- capture rereads outside the acquired coordinator
- missing source must be replaced by encoded defaults
- a corrupt or future source would become normally restorable without an
  explicit recovery policy
- an unbounded payload or allocation is required
- archive encoding or encryption extends the coordination lifetime
- custom adapter behavior must be invented inside the ordinary JSON path
- the card expands into ZIP, retention, restore, or host UI

## Completion Evidence

- added explicit include, exclude-with-reason, and custom-adapter policy
- default-excluded secret, cache, runtime, and log domains
- added explicit all-registered and selected scopes with stable id ordering
- added strict bounded version-1 manifest, source evidence, SHA-256, limits,
  immutable payloads, and machine-readable receipts
- captured exact present, older-valid, future, and corrupt-readable bytes
  under the existing coordinator; missing files remain absent
- failed unreadable, unavailable, incomplete-policy, custom-adapter, and
  bounded-size cases without returning a partial snapshot
- proved pending debounce exclusion, forced-flush inclusion, guard release,
  and helper-process mutation exclusion
- retained Rust 1.85 and kept ZIP, age, Tauri, async runtime, Svelte, Poodle,
  restore, and adapter execution out
- full validation recorded in the batch log

## Next Task

Card 006 is ready. Stop before ZIP implementation because this card does not
auto-start its successor.
