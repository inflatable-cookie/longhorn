# Diagnostics Seam And Swallow Instrumentation

Date: 2026-08-03
Card: 145
Roadmap: g02.005

## Result

`longhorn-core` now owns one process-wide best-effort diagnostics seam;
all 23 audited tolerated-failure sites report through it. Default behavior
is exactly the historical silent tolerance.

## Shape

- `BestEffortDiagnostics` trait plus `install_best_effort_diagnostics`
  (first installation wins) and `report_best_effort_failure` in
  `longhorn-core`; no new crate, no new dependency, no composition-API
  break.
- Instrumented: changed-event emits (notifications, history, history-tree,
  operation ×2, transfer), mutation-hint emits (settings ×2, command ×2),
  native-content adapter runtime-event/detach/teardown/close paths
  (backing-surface ×4, isolated-window ×2, child-view ×2), restore journal
  cleanup (ordinary ×2, grouped ×1), and storage-transition temporary
  cleanup (NotFound excluded).
- Deliberate non-sites: windowing wake delivery (already reported through
  `WindowLifecycleReporter` since Card 139), idempotent `app.manage`
  installation, and unused-binding idioms.
- `longhorn-core` added as a direct dependency of tauri-history,
  tauri-history-tree, tauri-command, and tauri-notifications (previously
  transitive); no crate or package count change.

## Exact Evidence

- seam test proves silent-before-install, first-install-wins, and exact
  area/detail capture
- 23 `report_best_effort_failure` call sites; zero unjustified `let _ =`
  swallows remain in production paths
- workspace all-targets check and Clippy pass
