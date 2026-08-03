# Retag Coherence, Install Safety, And Race Closure

Date: 2026-08-03
Card: 140
Roadmap: g02.002

## Result

Retag now migrates the whole lifecycle identity, installation fails typed
before any mutation, and the six recorded lifecycle races are closed.

## Shape

- `WindowLifecycleCoordinator::retag` moves pending capture, debounce,
  capture generation, and apply expectation to the new identity, merging an
  apply expectation already registered under the new id (newest generation
  wins), and returns re-schedule directives; the Tauri host executes them so
  pending deadlines deliver under the new id. Old-id wakes fail as unknown
  and count as superseded. `release` removes state without directives.
- `install_window` validates the label into a `HostWindowHandle` before map
  insertion and returns the new `InvalidWindowLabel` error; no listenerless
  state can exist.
- Destroy racing a concurrent event no longer resurrects coordinator state:
  the host releases state recreated by an event that lost the race and
  returns `UnknownWindow`.
- Reveal: a lost page-ready/converged race is recorded (`reveal_retry`) and
  the failed in-flight attempt retries once; multi-window reveal advance
  continues past destroyed windows instead of aborting the batch.
- `retained_normal` write-back is compare-and-keep-newest, so a concurrent
  `register_apply` value can no longer be overwritten by an older unlocked
  capture.
- Composition apply uses a drop guard: panic or poison during the injected
  apply restores the registry (poison recovered by whole-value replacement)
  and resets the phase — the host can no longer end up permanently `Busy`.
- Dynamic window registration captures a best-effort initial normal
  placement, so maximize-before-first-capture windows still persist.

## Exact Evidence

- coordinator retag test proves pending generation and deadline fire under
  the new identity; merge test proves tracked-target retag and release
- oversized-label install returns `InvalidWindowLabel` with no installed
  state
- protected-primary retag composition fixture passes through the new
  coordinator migration path
- windowing 45 tests, tauri-windowing 49 tests, Clippy, and workspace
  all-targets check pass
