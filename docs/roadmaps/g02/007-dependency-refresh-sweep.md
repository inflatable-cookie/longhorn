# g02.007 Dependency Refresh Sweep

Status: complete
Owner: Tom
Updated: 2026-08-04
Governing refs: contracts 001, 004, 010, and 012; g02 candidate runway
Depends on: g02.006

## Outcome

Close the compatible transitive drift and evaluate each held-back pinned
crate against its conformance suite, so distribution candidate v2 freezes a
current graph instead of re-freezing a stale one.

## Generation Runway

Seventh g02 milestone, first Tier A candidate. Ordered before g02.008.

## Execution Plan

### Batch 1. Compatible drift and held-back evaluations

- [x] [Card 148](batch-cards/148-dependency-refresh-sweep.md)
  applies the compatible `cargo update`, then evaluates rusqlite, zip,
  ts-rs, and sha2 individually with their conformance suites

## Goals

- [x] transitive drift closed within existing version ranges
- [x] each held-back pin either bumped with green conformance or recorded
  as deliberately retained with a reason
- [x] backup-archive determinism, SQLite adapter proofs, and generated
  bindings remain exact

## Acceptance Criteria

- [x] `cargo update` drift applied; workspace tests, Clippy, and full
  `effigy qa` pass
- [x] rusqlite, zip, ts-rs, and sha2 each have a bump-or-retain decision
  with evidence
- [x] no protocol fixture or golden archive changes without an explicit
  regeneration receipt

## Explicit Non-goals

- new candidate receipt (g02.008 owns it)
- tauri major-line changes
- consumer repository edits

## Next Task

Promote g02.008 when Card 148 closes.
