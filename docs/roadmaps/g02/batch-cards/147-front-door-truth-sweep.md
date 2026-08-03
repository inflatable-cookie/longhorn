# 147 Front-door Truth Sweep

Status: complete
Owner: Tom
Roadmap: g02.006 batch 2
Governing refs: contracts 001 and 012; research memo 018
Depends on: Card 146
Auto-start next card: no
Completed: 2026-08-03

## Objective

Return every prose front door to repo truth after the g01 closeout drift and
record whatever g02 work has landed by then.

## Scope

- `README.md` current-state section
- `CHANGELOG.md` counts and missing entries
- `docs/contracts/contract-index.md` readiness and `Updated:` headers
- `docs/roadmaps/g01/batch-cards/README.md` index and stale pointer
- `docs/reference/api-surface.md` regeneration

## Steps

1. Rewrite `README.md` current state: g01 complete through g01.020, g02
   active, no per-card narrative backlog.
2. Fix `CHANGELOG.md`: 18 packages / 38 crates, add the fork-tree production
   layer and any landed g02 entries.
3. Refresh contract-index readiness lines and correct stale `Updated:`
   headers (contract 004 at minimum).
4. Move Card 074 to Complete in the g01 batch-card index and delete its
   divergent next-task pointer; the generation index stays the only live
   pointer.
5. Regenerate `docs/reference/api-surface.md` via the card-126 generator and
   confirm the check task passes.

## Acceptance Criteria

- no front door contradicts file state or the generation index
- exactly one live next-task pointer exists
- `check:api-reference-card126`, docs QA, and full `effigy qa` pass

## Evidence Required

- front-door diff summary in the batch log
- regeneration and QA receipts

## Stop Conditions

- regeneration reveals an undocumented public-surface change that needs its
  own card

## Evidence

- README, CHANGELOG, contract-index, contract 004 header, and g01
  batch-card index all match repo state; one live next-task pointer
- `docs/reference/api-surface.md` regenerated; its check passes
- log: `docs/logs/2026-08/03-qa-selectors-package-hygiene-and-front-doors.md`

## Next Task

g02 planning checkpoint: characterize the next shared gap from consumer
evidence or extend the runway.
