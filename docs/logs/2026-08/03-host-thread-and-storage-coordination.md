# Host Thread And Storage Coordination

Date: 2026-08-03
Cards: 143-144
Roadmap: g02.004

## Result

All 22 storage-heavy Tauri commands (config 13, settings 4, command 5) are
async over `spawn_blocking`; the coordination lock wait carries an explicit
thread-budget contract; bare `ConfigStore::load` self-heals terminal restore
journals; coordination poison recovery is a documented decision, not an
accident.

## Shape

- Command wrappers clone the service `Arc` and caller label and run the
  service call on the blocking pool. Join failure maps to the existing
  `state_unavailable` code; command names, payloads, receipts, and error
  wire shapes are unchanged.
- `wait_for_retry` documents that callers own the thread budget: UI hosts
  reach it only from blocking-pool threads.
- `ConfigStore::load` on `Active` restore state now distinguishes an
  ordinary journal already in `succeeded`/`rolled-back` phase — a
  crash-after-completion artifact — and heals it through the same
  `recover_guarded` path coordinated load-sets use, non-blocking
  (zero-timeout acquire; contention leaves the journal alone). Grouped
  journals never qualify; mid-flight phases still block.
- Poison decision reversed after evidence: typed `Poisoned` failure broke
  the grouped-restore crash tests, which panic inside adapters while holding
  the coordinator and then boot-recover in-process. The process lock guards
  pure exclusion over on-disk state with its own journals, so recovery is
  correct; both recovery sites now say why.

## Exact Evidence

- parked-service test proves contended service work runs on blocking
  threads while the async executor stays responsive
- crash-phase regression: `applying` journal still blocks bare loads;
  `succeeded` and `rolled-back` journals self-heal on bare load, cleaning
  the journal and returning `Ready`
- config 132 tests, tauri-config/settings/command suites, workspace Clippy,
  and fmt pass; invoke wire surface unchanged
