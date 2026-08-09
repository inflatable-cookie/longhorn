# Debounced Mutation Contract Promotion

Date: 2026-07-28  
State: complete research and planning batch

## Outcome

- audited Loophole, Nucleus, Soundcheck, Split-shell, and Jetstream debounce
  behavior
- separated reusable trailing-edge scheduling from donor-owned threads,
  renderer timers, and close policy
- promoted bounded typed intent instead of staged whole-domain snapshots
- required every flush to apply intent to a fresh value under card-002
  coordination
- selected an injected monotonic clock and host-owned wakeups
- made retry explicit and retained unpublished intent
- separated pre-replacement failure from known publication with failed
  durability
- defined generation receipts, semantic no-op, explicit discard, and aggregate
  shutdown flush
- recompiled card 003 as the only ready implementation card

## Donor Findings

Loophole has the strongest pure scheduling specimen: 200 ms trailing debounce,
caller-driven readiness, effective staged reads, semantic dedupe, and forced
flush. It stages a whole snapshot and removes it before I/O, so a failed write
loses intent and an intervening process update can be overwritten.

Nucleus and Soundcheck use 300 ms worker loops and close acknowledgements.
Their acknowledgements do not contain ordinary write failure. Soundcheck does
reread the full settings document before changing only the window projection,
which supports fresh-value intent application.

Split-shell uses a 200 ms renderer timer and flushes on gesture end, blur, and
teardown. Errors are visible, but rejected saves are not retained and teardown
cannot await publication.

Jetstream supplies no reusable debounce mechanism.

## Decision

One lane keeps one bounded, coalesced typed intent. Coalescing must preserve
ordered application. The core owns deterministic state; the host owns timers
and execution.

Flush rereads under the existing store-wide lock. Equal encoded values skip
publication. Pre-replacement failures retain intent and require explicit
retry. Known replacement clears intent even when required directory durability
fails, preventing duplicate non-idempotent application.

Drop performs no I/O. Hosts explicitly await single or aggregate forced flush
and receive complete per-domain outcomes.

## Scope Kept Out

- Tauri lifecycle and close policy
- threads, async runtimes, and automatic retry
- TypeScript, Svelte, and Poodle bindings
- persisted write-ahead intent
- cross-domain transactions
- backup, restore, and consumer migration

## Posture

`strict-ready`

Card 003 is ready. Its `Auto-start next card: no` boundary remains active.

## Next

Execute card 003 when implementation resumes.
