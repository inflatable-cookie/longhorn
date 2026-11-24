Isolated-window host destruction retires the generation before bounded owner
teardown and keeps an unresolved owner available for retry. Failed isolated
owners stop admitting runtime events. Isolated-window and child-view callbacks
report `ForeignIsland` for a mismatched island.

### Breaking

- `take_requests` now returns `AdmittedIsolatedContentRequest` values. Inspect
  the helper payload with `request()` and pass the admitted value to
  `decide_resize`. Drain the adapter queue to obtain the admission revision.
- Construct `HelperMessageKind::Progress` and
  `IsolatedWindowRuntimeEventKind::Progress` phases with
  `IsolatedWindowStartupPhase::new`. Protocol v1 still uses a JSON string and
  now rejects phases above 128 UTF-8 bytes. Keep helper phase text within that
  bound.
- Handle `ForeignIsland` for cross-island callbacks and `FailedGeneration`
  for further callbacks after isolated helper loss.
