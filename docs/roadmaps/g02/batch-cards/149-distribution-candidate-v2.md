# 149 Distribution Candidate V2

Status: active
Owner: Tom
Roadmap: g02.008 batch 1
Governing refs: contracts 001, 003, 012, and 013; Card 127 receipt
Depends on: Card 148
Auto-start next card: no

## Objective

Freeze the second private compatibility candidate over the refreshed graph
and clear the two deferrals parked behind the Card 127 receipt.

## Scope

- bridge `@longhorn/tauri` demotion to optional peer (package test,
  topology artifact proof, proof consumers)
- new candidate fixture, verifier, and receipt superseding Card 127
- refreshed 18-package/38-crate counts and dependency graph
- diagnostics-seam adoption section in the composition guides

## Steps

1. Demote the bridge peer and update the three pinned evidence layers.
2. Regenerate the candidate fixture and verifier over the current tree;
   archive the Card 127 receipt with a supersession pointer.
3. Re-run the packaged candidate proofs.
4. Add diagnostics-seam adoption guidance with one consumer-shape example.
5. Full `effigy qa` plus the candidate verifier.

## Acceptance Criteria

- optional-peer shape asserted end to end
- candidate verifier passes against the live tree
- superseded receipt archived, not rewritten
- full `effigy qa` passes

## Evidence Required

- new receipt digest and verifier receipts
- supersession record
- QA receipts

## Stop Conditions

- a consumer `file:` install breaks on the peer shape
- Poodle artifact set drift forces a coordinated re-freeze

## Blocker

Receipt generation requires clean consumer graph sources; nucleus has a
dirty `apps/desktop/src-tauri/Cargo.toml` from its in-flight integration
thread. Consumer repos are read-only from Longhorn. Resume:
`effigy generate:private-candidate-card149` once the nucleus tree is clean,
then the proof, docs, and closeout steps.

## Next Task

g02 planning checkpoint.
