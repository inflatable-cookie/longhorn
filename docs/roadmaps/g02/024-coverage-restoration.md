# g02.024 Coverage Restoration

Status: complete
Completed: 2026-08-15
Owner: Tom
Updated: 2026-08-15
Governing refs: contract 002; contract 010; contract 019; memo 023
Depends on: none (Card 179's absorption is landed)

## Outcome

The test surface matches the code surface again. The layout behavior Card 179
moved into `longhorn-surfaces` is tested where it lives; the two newest
protocols carry the golden fixtures the thirteen older ones have; the three
hand-rolled untrusted-input parsers face generated input; and the hand-written
seams the audit found ungated are gated.

## Generation Runway

Memo 023's largest coverage finding is not a missing test — it is a deleted
one: Card 179 removed the layout suites with the crates and never ported them,
so the mutation semantics contract 002's absorbed sections specify are
untested. Everything else in this milestone is the same shape: surfaces that
grew faster than their evidence.

## Planning Gaps

- **`BoundedLayoutReplayStore` has no callers and no tests** while the
  held-surface register claims contract tests exercise it. Card 211 either
  wires replay into a real caller or deletes the store and corrects the
  register. That is a small product call — keep or cut — named here rather
  than buried.

## Execution Plan

### Batch 1. The post-179 hole

- [x] [Card 211](batch-cards/211-port-the-layout-suites.md): port the deleted
  layout-model and layout-config suites onto `longhorn-surfaces` and
  `longhorn-surfaces-config`; dispose of `BoundedLayoutReplayStore`; fix the
  stale container comment in the presentation test. **Landed 2026-08-15** —
  all 56 deleted tests recovered, and the port was rename-only: the
  divergence list is empty, so the absorption preserved the specified
  semantics. Replay is tested again; its absence of production callers is the
  register's fact to carry.

### Batch 2. Protocol evidence

- [x] [Card 212](batch-cards/212-licence-update-fixtures-and-seam-strings.md):
  golden fixtures for the licence and update protocols; the boundary test
  derives its domain list instead of transcribing it; the `longhorn-tauri`
  invoke/event strings get a conformance check against the Rust constants;
  dead `svelte-shims.d.ts` deleted. **Landed 2026-08-15** — fixtures generated
  and gated, boundary test derived (proven with a planted domain),
  `check:tauri-seam-strings` in `qa` (proven red both directions).

### Batch 3. Generated input

- [x] [Card 213](batch-cards/213-fuzz-the-three-parsers.md): property/fuzz
  coverage for the zip backup inspector, `parse_utc_timestamp`, and the
  history envelope decoders — `proptest` is already a workspace dependency.
  **Landed 2026-08-15** — six properties at 64 cases each, ~0.2s added, zero
  findings. The parsers held.

### Batch 4. Uniform coverage

- [x] [Card 214](batch-cards/214-port-parity-and-keyring-coverage.md): tests
  for the five untested `longhorn-tauri` raw ports; settings-navigation joins
  the parity fixture; bridge-job listeners gain a failure channel and
  malformed-event tests; keyring contract tests get a mock backend; the
  bindings generator's untested lanes get unit coverage. **Landed
  2026-08-15** — ports pinned by literal strings, `onFailure` shipped,
  keyring mock proves locked-is-never-empty off-platform, poodle-svelte runs
  from its own directory. One finding out of scope: the two settings-sidebar
  tiers disagree on module prefixes (recorded on the card; contract 013
  conversation).

## Dependency Shape

```text
memo 023 coverage gaps 1, 2, 4, 5, 6, 7, 10 + TS lane M1-M3
 └─ 024 coverage restoration
     ├─ 211 layout suites        (independent; largest)
     ├─ 212 fixtures + seams     (independent)
     ├─ 213 fuzz parsers         (independent)
     └─ 214 ports + parity       (independent)
```

Four independent cards; order by release risk, 211 first.

## Goals

- [x] no behavior specified in a contract is untested because its crate moved
- [x] every protocol has the neutral fixture neither side authors
- [x] every hand-written seam string has a mechanical check
- [x] the three untrusted parsers meet input they were not written against

## Acceptance Criteria

- [x] panel-mutation commands (`CreatePanelInstance`, `MovePanel`, sizing
  slots) have tests where the engine lives
- [x] `fixtures/licence/protocol-v1.json` and `fixtures/update/protocol-v1.json`
  exist and are gated like the other thirteen
- [x] the boundary test fails if a domain is added without updating it —
  because it no longer needs updating
- [x] the fuzz targets run in `qa` at a bounded iteration count

## Explicit Non-goals

- Coverage metrics or percentage targets. This milestone restores specific,
  named evidence — it does not instrument the tree.
- Re-testing what the audit verified strong (bridge authorization negatives,
  update verification, licence trust basis).

## Next Task

Milestone complete. The remaining audit suite: g02.025 docs spine (Card 215
first), g02.027 consolidation (Card 221). One open finding from this
milestone needs an operator decision: the settings sidebar label divergence
recorded on Card 214.

## Planning Checkpoint

After Card 211. The port will show whether contract 002's absorbed sections
describe the engine as built or as designed — any divergence there is a
contract correction, and Card 215 (g02.025) absorbs it.
