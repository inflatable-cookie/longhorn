# g02.003 Transfer Session Truthfulness

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 002, 010, and 011; research memo 018
Depends on: none

## Outcome

Make every transfer abort report session consumption truthfully, convert
post-publication assertions into the crate's own reconciliation evidence, and
close the client-binding lifecycle races in the Tauri transfer host.

## Generation Runway

Third g02 milestone. Bounded to `longhorn-surface-transfer` and
`longhorn-tauri-transfer`; coordinator and wire protocol semantics stay fixed.

## Execution Plan

### Batch 1. Consumed-session aborts and reconciliation evidence

- [x] [Card 141](batch-cards/141-consumed-abort-truth-and-reconciliation-evidence.md)
  marks post-consumption Surface-commit failures `.consumed()` and replaces
  post-publication asserts with `HostReconciliationRequired`

### Batch 2. Client binding lifecycle races

- [x] [Card 142](batch-cards/142-client-binding-races-and-ordered-events.md)
  closes the snapshot/destroy binding leak and orders client-changed
  emission against epoch advancement

## Goals

- [x] `session_consumed` matches coordinator state on every abort path
- [x] no reachable panic after durable Surface publication
- [x] destroyed windows cannot re-acquire client slots
- [x] renderers never observe an older epoch after a newer one

## Acceptance Criteria

- [x] Surface-commit binding/load failures after consumption serialize
  `session_consumed: true`; retry yields `SessionReplayed`, not a hang
- [x] container-mismatch after publication returns reconciliation evidence in
  release builds
- [x] snapshot racing destroy leaves no binding; capacity cannot be exhausted
  by cycles
- [x] transfer suites, packaged-proof regressions, and workspace QA pass

## Explicit Non-goals

- wire protocol version bump
- coordinator state-machine changes
- consumer repository edits

## Next Task

Promote Card 143 (g02.004).
