# 211 Port The Layout Suites

Status: complete
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.024 batch 1
Governing refs: contract 002 (absorbed sections); memo 023 (coverage gap 1)
Depends on: none
Auto-start next card: no

## Objective

The layout behavior Card 179 moved into `longhorn-surfaces` is tested where it
lives, and `BoundedLayoutReplayStore` is either wired to a caller or deleted.

## Why this exists

Card 179 deleted `crates/longhorn-layout/tests/layout_model*` (definitions,
donors, mutation/policy/replay, state, visibility — 12 files) and the
321-line `longhorn-layout-config` debounce suite (`dfa72456`: 2228 deletions).
Today `crates/longhorn-surfaces/src/layout/**` has zero inline tests;
`tests/surface_contract/` covers Surface topology but nothing references
`LayoutMutationCommand::CreatePanelInstance`/`MovePanel`/sizing slots;
`longhorn-surfaces-config` has no debounce or replay tests. The deleted
behavior is exactly what contract 002's absorbed sections (`:199-372`)
specify — specified, untested. Meanwhile `docs/reference/held-surface.md:35`
claims `apply_with_replay` is "exercised by contract tests"; those tests were
deleted with the crate and the store has zero references repo-wide.

## Scope

- `crates/longhorn-surfaces` — the layout mutation engine tests
- `crates/longhorn-surfaces-config` — debounce and replay tests
- `BoundedLayoutReplayStore` / `apply_with_replay` — disposition
- the stale comment at `tests/surface_contract/mutation/presentation.rs:16`

## Steps

1. Recover the deleted suites from git (`git show a4dda1f7`,
   `git show dfa72456`) and port them onto the absorbed module paths. The
   behavior was specified; the port should be mechanical where the engine
   kept the semantics.
2. Where the port is not mechanical — the engine diverged from the deleted
   tests' expectations — that divergence is a finding: record it for Card
   215's contract amendment rather than editing tests to pass.
3. `BoundedLayoutReplayStore`: wire `apply_with_replay` into a real caller
   with tests, or delete both and correct the held-surface register (Card
   216 owns the register edit; this card supplies the fact).
4. Fix the `presentation.rs:16` container comment while in the file.

## Do Not

- Rewrite the engine to match deleted tests without surfacing the divergence.
  The contract's absorbed sections are the arbiter.
- Port test *volume* for its own sake — port the specified behaviors.

## Result

All 56 deleted tests ported, and the port was mechanical in the way that
matters: **the divergence list is empty.** Every recovered test passed
against the absorbed engine after renames alone (`LayoutContainerId`→
`SurfaceId`, `LayoutDocument`→`SurfaceDocument`, and the rest of the Card 179
mapping) — the absorption kept the semantics the suites specified.

- `crates/longhorn-surfaces/tests/layout_model.rs` (+ support, definitions,
  donors, state, visibility, mutation/{success,failures,policy,replay,
  donors}) — 38 tests over the engine: commit semantics, exact-source
  preservation on rejection, instance-count policies, donor shapes.
- `crates/longhorn-surfaces-config/tests/layout_config.rs` (+ support,
  debounce, loading, mutation, backup) — 18 tests over the debounce lane
  (including cross-process coordination timeout), loading/migration, and
  backup policy.
- The stale "container inventory" comment in
  `surface_contract/mutation/presentation.rs` is fixed.

Replay disposition: the ported `mutation/replay.rs` exercises
`apply_with_replay` and `BoundedLayoutReplayStore` meaningfully — exact
replay, request-id conflict, bounded eviction — so the held-surface
register's "exercised by contract tests" line is true again. It still has no
production caller; that is the register's fact to carry, not a reason to
invent one.

Counts: longhorn-surfaces 24 → 62, longhorn-surfaces-config 10 → 28.

## Acceptance Criteria

- [x] panel-mutation commands, sizing slots, visibility, and policy have tests
  where the engine lives
- [x] the debounce lane is tested in `longhorn-surfaces-config`
- [x] `apply_with_replay` has tests — it has no caller, and the register says
  exactly that
- [x] every non-mechanical divergence is recorded — there were none

## Evidence Required

- the ported suites, green
- the divergence list, empty or written
- `effigy qa` green

## Stop Conditions

Stop if the port reveals the absorbed engine lacks behavior contract 002
specifies — that is a contract/code gap, not a test gap, and it re-plans.
