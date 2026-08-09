# Debounced Mutation And Explicit Flush

Date: 2026-07-28  
State: complete implementation batch

## Outcome

- added typed consumer strategies for coalescing, fresh-value application, and
  pending weight
- added bounded trailing-edge lanes with injected monotonic clocks
- added monotonic generations, stage receipts, bounded snapshots, and
  next-deadline observation
- added due, forced, discard, and stable-order aggregate flush
- added encoded semantic no-op detection inside coordinated mutation
- retained unpublished intent across every mutation error class
- suppressed automatic due retry after failure
- cleared known-published intent after durability failure
- kept the crate free of threads, async runtimes, Tauri, Svelte, and Poodle

## Public Boundary

`DebouncedMutation` borrows an existing registered store and domain.
`DebounceStrategy` keeps consumer intent and product schema outside Longhorn.
The strategy must preserve ordered application when coalescing and supply a
deterministic pending weight.

`DebounceClock` supplies monotonic process-local time. `SystemClock` is the
standard host clock; tests inject fake clocks. Longhorn reports the next
deadline but owns no wakeup mechanism.

Flush always enters coordinated fresh-value mutation. `ConfigStore::load`
continues to report persisted authority, not optimistic pending state.

## Failure Boundary

Pre-publication failure keeps the same generation in `retry-required`. Due
polling performs no further I/O. Forced flush retries, new accepted input
coalesces and resets the deadline, and explicit discard is observable.

Known atomic replacement clears pending intent even when required directory
durability fails. Retrying could apply a non-idempotent command twice.

Aggregate flush is stable by domain id, attempts every inserted lane after
partial failure, and is not a cross-domain transaction. Drop performs no I/O.

## Evidence

- 48 passing tests
- ten fake-clock and state-machine unit tests
- every uncommitted `MutationError` retains generation and intent
- publication tests cover failures before and after replacement
- intervening store and helper-process updates survive flush
- Loophole layout command fixture
- Nucleus bounded last-geometry fixture
- Split-shell presentation projection fixture
- semantic no-op, explicit discard, duplicate lane, wrong store, stable
  aggregate order, partial failure, and drop-with-pending proofs
- Rust 1.85 workspace check
- clean format, clippy, Effigy doctor, test plan, and full QA

## Platform Boundary

Scheduling state is process-local. A crash may lose the accepted but unflushed
interval. Consumers requiring immediate durability use direct mutation.

Host adapters remain responsible for timers, worker execution, shutdown
deadlines, and close policy. They must explicitly await forced flush before
teardown.

## Posture

`strict-paused`

Safe mutation is complete. Backup implementation is paused on archive,
encryption, consistent-snapshot, and atomic-restore decisions in card 004.

## Next

Research and promote card 004's backup/archive contract. Do not implement
backup before the gate closes.
