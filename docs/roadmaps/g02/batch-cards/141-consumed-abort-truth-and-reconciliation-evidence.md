# 141 Consumed-abort Truth And Reconciliation Evidence

Status: complete
Owner: Tom
Roadmap: g02.003 batch 1
Governing refs: contracts 001, 002, and 011; research memo 018
Depends on: none
Auto-start next card: no
Completed: 2026-08-03

## Objective

Serialize `session_consumed` truthfully on every Surface-commit abort and
replace post-publication assertions with the reconciliation evidence the
surrounding code already models.

## Scope

- `crates/longhorn-surface-transfer/src/commit/existing.rs`
- `crates/longhorn-surface-transfer/src/commit/provisioned.rs`
- abort serialization regression fixtures

## Steps

1. Mark post-consumption binding and `load_surface` failures `.consumed()` in
   both commit paths, mirroring `longhorn-transfer/src/panel/commit.rs`.
2. Replace the post-publication `assert_eq!` container checks with
   `HostReconciliationRequired` evidence, matching every comparable
   post-publication failure branch.
3. Add regressions: abort-after-consumption serializes
   `session_consumed: true` and a retry yields `SessionReplayed`;
   container mismatch after publication returns reconciliation evidence in a
   release-profile test.

## Acceptance Criteria

- no abort path can report an unconsumed session the coordinator marked
  `Attempted`
- no reachable panic after durable Surface publication
- transfer suites and workspace QA pass; wire fixtures unchanged except the
  corrected flag

## Evidence Required

- asymmetry regression receipts for both commit paths
- release-profile reconciliation receipt
- QA receipts

## Stop Conditions

- the corrected flag breaks a consumer renderer's recorded retry behavior
  (coordinate before landing)

## Evidence

- post-consumption binding/load failures marked `.consumed()` in both
  commit paths; regression proves `session_consumed: true` and terminal
  `SessionReplayed` on retry
- post-publication asserts replaced with `HostReconciliationRequired`
  evidence (provisioned path carries provision + publication)
- log: `docs/logs/2026-08/03-transfer-session-truthfulness.md`

## Next Task

Promote Card 142.
