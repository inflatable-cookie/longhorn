# 140 Retag Coherence, Install Safety, And Race Closure

Status: complete
Owner: Tom
Roadmap: g02.002 batch 2
Governing refs: contracts 001, 009, 010, and 017; research memo 018
Depends on: Card 139
Auto-start next card: no
Completed: 2026-08-03

## Objective

Make retag migrate the whole lifecycle state, make installation fail typed
before any mutation, and close the recorded lifecycle races.

## Scope

- `crates/longhorn-tauri-windowing/src/lifecycle/host.rs` retag and install
- `crates/longhorn-tauri-windowing/src/lifecycle/host/reveal.rs`
- `crates/longhorn-tauri-windowing/src/lifecycle/host/directives.rs`
  retained-normal write-back
- `crates/longhorn-tauri-windowing/src/composition/{host,assembly}.rs`

## Steps

1. `retag_window`: rename coordinator per-window state with the host entry,
   re-key pending capture/flush and their scheduled wakes, preserve
   `capture_generation`, and remove the old-id entry.
2. `install_window`: validate the label into `HostWindowHandle` before map
   insertion; return a typed error, never panic, never leave listenerless
   state.
3. Reveal: retain trigger evidence so a failed `reveal()` retries on the next
   advance; continue past per-window failures and return aggregated receipts.
4. `retained_normal`: guard the unlocked-capture write-back against a newer
   value installed by `register_apply` (compare-and-keep-newest).
5. `handle_lifecycle_event`: do not resurrect coordinator entries for
   destroyed windows (`or_default` after removal).
6. Composition host: add a drop/restore guard so poison or apply panic cannot
   leave the registry permanently `Busy`; let dynamic registration accept an
   initial normal placement so maximize-before-first-capture persists.

## Acceptance Criteria

- retag under pending work delivers under the new id with generation intact;
  no stale coordinator entries remain after destroy
- oversized-label install returns an error with zero installed state
- each closed race carries a regression test that failed before the fix
- windowing suites, composition proofs, and workspace QA pass

## Evidence Required

- retag-with-pending and generation-preservation test receipts
- race regression receipts
- QA receipts

## Stop Conditions

- retag semantics require consumer-visible id changes
- the initial-normal policy needs a product decision on default geometry

## Evidence

- coordinator `retag`/`release` with pending migration, apply-expectation
  merge, and new-identity deadline delivery
- typed `InvalidWindowLabel` pre-insert install validation
- reveal retry-on-race and continue-past-destroyed aggregation
- retained-normal compare-and-keep-newest write-back
- apply drop guard with poison recovery; dynamic initial-normal capture
- windowing 45 and tauri-windowing 49 tests, Clippy, workspace all-targets
  check, and full `effigy qa` pass
- log: `docs/logs/2026-08/03-retag-coherence-install-safety-and-race-closure.md`

## Next Task

Promote Card 141 (g02.003).
