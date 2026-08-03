# g02.006 QA And Docs Alignment

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001 and 012; research memo 018
Depends on: none

## Outcome

Make every QA selector resolve, close the aggregate-check and package
hygiene gaps, and return the prose front doors to truth after the g01
closeout drift.

## Generation Runway

Sixth g02 milestone. Independent of 001-005; may interleave. Docs sweep runs
last so it records whatever g02 work has landed.

## Execution Plan

### Batch 1. QA surface and package hygiene

- [x] [Card 146](batch-cards/146-qa-selectors-and-package-hygiene.md)
  fixes the dangling history fixture selectors, completes the bindings and
  client-ts aggregates, and settles peer-range, bridge-peer, workspace
  protocol, and cargo version conventions

### Batch 2. Front-door truth sweep

- [x] [Card 147](batch-cards/147-front-door-truth-sweep.md)
  rewrites the stale README state, contract-index readiness, g01 batch-card
  index, CHANGELOG counts, and regenerates the API surface

## Goals

- [x] every `effigy.toml` selector resolves its referenced paths
- [x] aggregates cover history-tree bindings and layout TS checks
- [x] one peer-range and internal-dependency convention across packages
- [x] front doors state g01 complete and g02 active with no competing
  next-task pointer

## Acceptance Criteria

- [x] `qa:northstar:g01-history-persistence` and
  `qa:northstar:g01-history-tree-persistence` pass (finding retracted:
  fixtures are crate-relative and existed)
- [x] bindings aggregate includes history-tree; `check:client-ts` includes
  layout
- [x] README, CHANGELOG, contract-index, and g01 batch-card index match repo
  state; regenerated `docs/reference/api-surface.md` is current
- [x] full `effigy qa` passes

## Explicit Non-goals

- package-manager publication (remains deferred)
- rusqlite major migration beyond a deliberate pin refresh decision
- consumer repository edits

## Next Task

g02 planning checkpoint: characterize the next shared gap or extend the
runway from consumer evidence.
