# g02.008 Distribution Candidate V2

Status: ready
Owner: Tom
Updated: 2026-08-04
Governing refs: contracts 001, 003, 012, and 013; Card 127 receipt; g02
candidate runway
Depends on: g02.007

## Outcome

Freeze a second private compatibility candidate over the refreshed graph,
superseding the Card 127 receipt: bridge `@longhorn/tauri` demoted to an
optional peer, truthful 18-package/38-crate counts, and re-frozen
commit-pinned proofs. Includes the diagnostics-seam adoption guidance.

## Generation Runway

Eighth g02 milestone, closes the Tier A lane.

## Execution Plan

### Batch 1. Candidate receipt and adoption guidance

- [ ] [Card 149](batch-cards/149-distribution-candidate-v2.md)
  demotes the bridge peer, regenerates the candidate receipt and proofs,
  and documents diagnostics-seam adoption

## Goals

- [ ] bridge main entry ships without a hard `@longhorn/tauri` dependency
- [ ] candidate receipt reflects the current package/crate sets and the
  refreshed dependency graph
- [ ] Card 127 receipt superseded, not silently rewritten
- [ ] consumers get one diagnostics-seam adoption reference

## Acceptance Criteria

- [ ] bridge package test, topology artifact proof, and proof consumers
  assert the optional-peer shape
- [ ] new candidate fixture and verifier pass; the superseded receipt is
  archived with a pointer
- [ ] full `effigy qa` passes

## Explicit Non-goals

- registry publication (still deferred)
- Poodle artifact set changes
- consumer repository edits

## Next Task

g02 planning checkpoint after the Tier A lane closes.
