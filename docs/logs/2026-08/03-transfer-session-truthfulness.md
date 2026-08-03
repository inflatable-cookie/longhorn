# Transfer Session Truthfulness

Date: 2026-08-03
Cards: 141-142
Roadmap: g02.003

## Result

Every Surface-transfer abort now reports session consumption truthfully,
post-publication container drift returns reconciliation evidence instead of
a release-profile panic, the snapshot/destroy client-binding leak is closed,
and client-changed events observe epoch order.

## Shape

- `commit_existing` and `commit_provisioned` mark post-consumption
  `bindings.get` and `load_surface` failures `.consumed()`, matching the
  panel path's `as_consumed` handling.
- Both post-publication `assert_eq!` container checks became
  `HostReconciliationRequired` evidence; the provisioned path attaches
  `ReconciliationRequired { provision, publication, failure }`. Unreachable
  through the real mutation engine (which retains the binding), so covered by
  the type change rather than an engine-violating fixture.
- `TransferHandlerAssembly::snapshot` rechecks caller liveness after binding
  under the state lock and undoes its own binding when a destroy won the
  race; a destroy ordered later removes the binding itself.
- `longhorn_transfer_snapshot` serializes snapshot acquisition with event
  emission behind a state-level mutex so concurrent snapshots cannot deliver
  out of epoch order, and emit failure no longer hides already-advanced
  authority behind an error — the invoke result is the authoritative
  delivery. The advisory emit swallow is a named Card 145 instrumentation
  site.

## Exact Evidence

- consumed-binding regression: post-consumption `UnknownHostBinding` abort
  serializes `session_consumed: true`; replay of the same session returns
  `SessionReplayed`; the Surface document stays at baseline
- race regression: 32 raced snapshot/destroy cycles against a
  client-window capacity of 4 all fail with the runtime error, never
  capacity exhaustion; recovery snapshot succeeds with a monotonic epoch
- surface-transfer 12 tests, transfer 31 tests, tauri-transfer 12 tests,
  Clippy, and workspace all-targets check pass; full `effigy qa` green
