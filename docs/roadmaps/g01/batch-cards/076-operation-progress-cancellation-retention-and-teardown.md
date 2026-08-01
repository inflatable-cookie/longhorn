# 076 Operation Progress, Cancellation, Retention, And Teardown

Status: complete
Owner: Tom
Roadmap: g01.012 batch 1
Governing refs: contracts 001, 012, and 015; research memo 016
Depends on: Card 075
Auto-start next card: no
Completed: 2026-07-31

## Objective

Complete the pure operation authority with bounded progress, receipted
cancellation, exact retention, retry lineage, and explicit host teardown.

## Scope

- indeterminate, unit, and normalized overall progress
- phase identity and bounded presentation label
- progress sequence and non-regression
- accepted, already-requested, unsupported, and terminal cancellation receipts
- queued cancellation and running cancellation races
- finite count and encoded-weight retention
- explicit terminal dismissal and eviction receipts
- retry lineage through a new operation id
- controlled teardown and consumer-proven interruption
- renderer-detach non-cancellation fixture

## Out Of Scope

- executor or cancellation-token implementation
- queue pause, ordering, or concurrency
- durable scheduler or automatic restart resumption
- product warnings, logs, reports, artifacts, or recovery steps
- renderer clients and notifications

## Steps

1. Add bounded progress projections and sequence validation.
2. Enforce overall non-regression and phase-local reset rules.
3. Add revision-bound cancellation request commands and receipts.
4. Prove success, failure, and cancellation race terminals.
5. Add exact finite retention and explicit terminal dismissal.
6. Add `retry_of` without terminal reopening.
7. Add controlled teardown resolution for each non-terminal state.
8. Prove renderer detach leaves host work unchanged.
9. Re-run both donor fixtures and dependency audits.

## Acceptance Criteria

- progress rejects non-finite, overflowing, regressing, and late updates
- phase-local reset requires a new phase id
- cancellation acceptance never claims terminal cancellation
- repeated cancellation is idempotent
- stale revision and epoch leave exact state unchanged
- active operations never evict
- every terminal eviction and dismissal is receipted
- retry creates a distinct operation
- controlled teardown accounts for all non-terminal work
- remount does not cancel Soundcheck-shaped work

## Evidence Required

- progress validation matrix
- three cancellation-race traces
- queued-cancel and unsupported-cancel fixtures
- count/weight retention and overflow fixtures
- retry-lineage fixture
- teardown/interruption matrix
- exact-state failure assertions
- focused Rust, clippy, docs, formatting, and Effigy checks

## Stop Conditions

- executor acknowledgement cannot remain consumer-supplied
- progress requires product payload serialization
- retention can silently evict active work
- process restart requires an uncontracted durable scheduler
- renderer teardown must own host cancellation

## Next Task

Execute ready Card 077. Generate the checked protocol and compose direct,
Tauri, and bridge-domain transports.
