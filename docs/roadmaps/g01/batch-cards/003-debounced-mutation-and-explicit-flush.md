# 003 Debounced Mutation And Explicit Flush

Status: complete  
Owner: Tom  
Completed: 2026-07-28  
Roadmap: g01.002 batch 2  
Governing refs: contracts 001, 004, and 012; research memo 005  
Auto-start next card: no

## Objective

Add bounded trailing-edge configuration mutation that reapplies typed intent
to a fresh coordinated value and provides truthful retry, receipt, and
shutdown behavior without taking ownership of a host runtime.

## Scope

- one opt-in lane per scheduler, store, and registered domain
- consumer-owned typed intent, coalescer, applicator, weight, and limit
- transactional ordered coalescing with trailing-edge deadlines
- injected monotonic clock and next-deadline observation
- due flush, forced domain flush, and stable-order aggregate flush
- fresh-value application through card-002 coordination
- encoded semantic no-op detection under the same guard
- generation-based stage, flush, failure, discard, and snapshot records
- explicit retry-required state with no automatic retry
- commit-aware pending retention across publication failures
- bounded bookkeeping and pending memory
- deterministic unit and integration tests without sleeps

## Public Behavior

Debounce is explicitly configured for a consumer-owned domain use. It is not a
default `ConfigStore` write path.

Each lane accepts one typed intent shape and supplies:

- deterministic `coalesce(previous, next)`
- `apply(intent, fresh_value)`
- deterministic pending weight and maximum
- trailing-edge delay
- finite card-002 lock timeout and durability requirement

Exact Rust names may vary. The public behavior may not.

The required coalescing law is:

```text
apply(coalesce(a, b), fresh) == apply(b, apply(a, fresh))
```

for every valid fresh value. Coalescing builds a candidate before commit.
Failure or overflow rejects only the new stage and keeps the previous pending
intent, generation, and deadline.

An accepted stage receives the next generation and resets the deadline. Its
immediate receipt states whether it opened the lane or coalesced into pending
work. The scheduler retains no per-stage future or waiter.

The scheduler uses an injected monotonic clock. It exposes its next deadline
but creates no timer, thread, task, executor, or Tauri dependency. Due flush
acts only when the deadline has passed. Forced flush bypasses timing.

Flush acquires the existing store-wide coordinator and then rereads the
domain. It applies pending intent to that value, validates, encodes, and
compares encoded current and candidate values under the same guard. Equal
values clear pending state without replacing the file. Changed values use the
existing atomic publication path.

Failure behavior follows the card-002 replacement boundary:

- failures before replacement retain pending intent and enter
  `retry-required`
- due polling in `retry-required` performs no I/O
- forced flush retries
- a new accepted stage coalesces with retained intent and starts a new delay
- explicit discard clears unpublished intent and returns a receipt
- known replacement followed by durability failure clears pending intent and
  reports publication plus durability failure

Every flush result carries domain id and generation. Outcomes distinguish no
pending work, not due, unchanged, published, pre-publication failure retained,
and published with durability failure. A terminal result covers all accepted
stages through its generation. A bounded snapshot exposes pending generation,
deadline, retry-required state, and last terminal result.

Aggregate forced flush visits domain ids in stable order, attempts all pending
lanes after partial failure, and returns one result per attempted lane. It does
not promise cross-domain atomicity.

Drop performs no I/O. Host code must call and await forced flush before
teardown. The crate returns enough detail for a later Tauri adapter to delay,
cancel, retry, or continue close without treating failure as success.

`ConfigStore::load` continues to return authoritative persisted state. The
consumer owns optimistic UI projection.

## Out Of Scope

- background worker, async runtime, Tauri lifecycle, or close-window policy
- TypeScript/Svelte bindings and Poodle UI
- backup, migration rewrite, archive, retention, and restore
- server or multi-machine coordination
- cross-domain transactions
- consumer schemas and migration
- general job scheduling or retry backoff
- persisted write-ahead intent

## Steps

1. Add clock, policy, intent-budget, generation, state, outcome, and error
   types with a fake-clock test fixture.
2. Add one typed lane with transactional coalescing, checked trailing
   deadlines, bounded weight, snapshots, and explicit discard.
3. Extend the internal coordinated mutation path with encoded no-op detection
   while preserving the existing immediate `mutate` contract.
4. Apply lane intent only inside fresh-value coordinated mutation. Map every
   failure to pre- or post-replacement state.
5. Add due and forced flush. Suppress due retries after failure while allowing
   explicit retry and new-stage rescheduling.
6. Add a heterogeneous flush set with duplicate-key rejection, stable
   domain-id ordering, complete partial-failure results, and no destructor I/O.
7. Prove donor-shaped projection replacement, ordered patch composition,
   intervening process mutation, failure retention, commit-aware clearing, and
   bounded memory.
8. Run the complete batch validation and record closeout evidence. Do not
   start backup/archive work from this card.

## Acceptance Criteria

- fake time proves trailing-edge reset, not-due, due, and forced flush without
  sleeping
- one lane retains one coalesced intent and bounded terminal metadata
- last-value geometry/presentation replacement stays within its configured
  weight
- ordered partial patches satisfy sequential application after coalescing
- coalescer error and weight overflow preserve earlier state and deadline
- duplicate store/domain lane registration is typed
- another store instance mutating between stage and flush keeps its unrelated
  change
- another process mutating between stage and flush keeps its unrelated change
- unchanged encoded output skips atomic replacement and clears pending
- all coordination, patch, validation, encoding, serialization, and
  pre-replacement publication failures retain the same pending generation
- due polling after failure performs no write and returns retry-required
- forced retry publishes retained intent exactly once
- a post-replacement durability failure reports known publication and clears
  pending intent
- new input after failure coalesces with retained intent and starts a new
  deadline
- explicit discard is observable and performs no write
- receipts reconcile every accepted stage through terminal generation without
  per-stage waiters
- aggregate close flush uses stable order, attempts every lane, and reports
  partial failure
- dropping a lane or flush set performs no write
- package graph still contains no Tauri, async-runtime, Svelte, Poodle,
  Surface, or consumer dependency

## Evidence Required

- focused fake-clock unit tests for state transitions and checked deadlines
- property or table tests for ordered coalescing and budget rejection
- focused mutation tests for semantic no-op and commit-boundary mapping
- same-process and helper-process intervening-mutation fixtures
- failure-injection tests before and after atomic replacement
- aggregate partial-failure and drop-with-pending tests
- Loophole layout, Nucleus/Soundcheck geometry, and Bovine presentation-shaped
  conformance fixtures
- Rust 1.85 workspace check
- `effigy doctor`
- `effigy test --plan`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`
- batch log with crash-loss and host-lifecycle limits

## Stop Conditions

- staging requires blind whole-domain replacement
- intent cannot be bounded without an unbounded closure or waiter queue
- flush bypasses the card-002 coordinator or rereads before acquisition
- semantic no-op detection releases the guard before comparison
- failure state cannot distinguish whether atomic replacement happened
- safe retry would duplicate a known-published non-idempotent intent
- implementation requires a Tauri, Tokio, browser, or Svelte runtime
- aggregate flush expands into cross-domain transactions
- the card expands into backup, migration rewrite, or consumer migration

## Completion Notes

- added typed consumer strategies for intent coalescing, application, and
  deterministic pending weight
- added bounded trailing-edge lanes with injected monotonic clocks, monotonic
  generations, next-deadline observation, and no runtime dependency
- added transactional coalescing, typed overflow/deadline failures, bounded
  snapshots, and explicit discard
- added due and forced flush over a fresh card-002 coordinated reread
- added encoded semantic no-op detection without changing immediate
  `ConfigStore::mutate` behavior
- retained every pre-publication failure in `retry-required` and suppressed
  automatic due retry
- cleared known-published intent after durability failure
- added stable domain-order aggregate flush with duplicate and wrong-store
  rejection
- kept drop free of filesystem I/O
- retained Rust 1.85 and a Tauri/async/UI-free package graph

## Evidence

- 48 passing unit, integration, helper-process, and acceptance tests
- fake-clock trailing-edge, rescheduling, deadline, and retry proofs
- ordered partial-patch and bounded last-value replacement proofs
- exhaustive uncommitted `MutationError` retention mapping
- semantic no-op and explicit discard proofs
- known post-publication durability-failure clearing proof
- aggregate stable-order partial-failure proof
- Loophole layout, Nucleus geometry, and Bovine presentation fixtures
- intervening same-process and helper-process fresh-value proofs
- drop-with-pending no-write proof
- `cargo +1.85.0 check --workspace --all-targets`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy doctor`
- `effigy test --plan`
- `effigy qa`

## Next Task

Research and promote the backup archive, encryption, snapshot, and atomic
restore decisions named in card 004. Do not implement backup while that card
is paused.
