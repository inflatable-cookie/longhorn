# 144 Restore Self-heal And Poison Consistency

Status: complete
Owner: Tom
Roadmap: g02.004 batch 2
Governing refs: contracts 001 and 004; research memo 018
Depends on: Card 143
Auto-start next card: no
Completed: 2026-08-03

## Objective

Let bare loads recover terminal restore phases the way coordinated load-sets
already do, and surface coordination poison as typed errors.

## Scope

- `crates/longhorn-config/src/backup/restore/{execution,journal,recovery}.rs`
- `crates/longhorn-config/src/coordination.rs` poison handling
- `ConfigStore::load` recovery path

## Steps

1. Route bare `ConfigStore::load` through the existing `recover_guarded`
   self-heal when the journal holds `Succeeded`/`RolledBack`, instead of
   returning `Unavailable(RestoreActive)`.
2. Keep genuinely active restores blocking loads exactly as today.
3. Map coordination mutex poison to the workspace's typed `Poisoned` error
   instead of `into_inner()` continuation.
4. Regressions: crash-after-success then bare load returns data;
   crash-mid-restore still blocks; poison surfaces typed.

## Acceptance Criteria

- terminal journal phases never wedge plain loads
- active-restore protection unchanged
- config suites, grouped-restore suites, and workspace QA pass

## Evidence Required

- crash-phase matrix receipts
- poison behavior receipts
- QA receipts

## Stop Conditions

- self-heal on load would race a concurrent restore in a way the journal
  cannot arbitrate

## Evidence

- bare-load self-heal of terminal ordinary journals via `recover_guarded`;
  mid-flight phases and grouped journals still block; crash-phase regression
- poison decision reversed with recorded rationale: recovery is load-bearing
  for in-process boot recovery after panicked adapters; both sites documented
- log: `docs/logs/2026-08/03-host-thread-and-storage-coordination.md`

## Next Task

Promote Card 145 (g02.005).
