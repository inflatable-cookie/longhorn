# 069 Forkable History Promotion Decision And Closeout

Status: complete
Owner: Tom
Roadmap: g01.011 batch 4
Governing refs: contracts 001, 003, 008, and 012; research memo 015
Depends on: Card 068
Auto-start next card: no
Authorized: 2026-07-31
Completed: 2026-07-31

## Objective

Review the private fork prototype, choose promote, retain as research, or
reject, update every canonical boundary, and close g01.011 without implying
unimplemented branch behavior.

## Decision

`Promote` the proven fork-tree semantics into planned g01.017. Retain the
Card 068 workspace as private executable research until production artifact
proof. Do not publish it, add it to the root workspace, or enable branch mode
in Loophole.

## Scope

- prototype evidence review
- Loophole value and migration-risk review
- public package and compatibility decision
- architecture, contract, inventory, topology, spec, roadmap, and research
  updates
- prototype retention or removal
- linear artifact revalidation
- g01.011 closeout and next runway pointer

## Decision Outcomes

### Promote

Record the accepted graph semantics and compile a later implementation lane.
Do not call the prototype production or migrate Loophole in this card.

### Retain

Keep the measured prototype as non-publishable research with exact unmet gates.
Linear history remains the only public package.

### Reject

Remove or archive the prototype, record why the complexity does not justify a
shared branch system, and keep linear history plus consumer-owned versions.

## Out Of Scope

- silently promoting the prototype
- branch production implementation
- Loophole donor writes
- g01.012 execution
- g01.015 migration

## Steps

1. Review every Card 068 gate and benchmark.
2. Compare branch value against linear history plus consumer versions.
3. Choose exactly one decision outcome.
4. Update contract 008 and package topology truthfully.
5. Update research residuals, inventory, spec, and later migration gates.
6. Retain, archive, or remove prototype artifacts per the decision.
7. Re-run linear artifact and dependency proof.
8. Record g01.011 retained, changed, rejected, and deferred outcomes.
9. Run full Effigy QA.
10. Close g01.011 and set one next-task pointer.

## Acceptance Criteria

- exactly one decision is recorded with evidence
- public package claims match implemented behavior
- no prototype artifact masquerades as a release package
- Loophole migration remains lossless and branch-disabled unless separately
  promoted and implemented
- project versions, collaboration, and event sourcing remain distinct
- all front doors and later gates agree
- linear artifact proof still passes
- full Effigy QA passes

## Evidence Required

- gate-by-gate prototype decision table
- performance and storage summary
- Loophole benefit and migration-risk analysis
- canonical docs diff
- prototype disposition proof
- linear artifact and dependency revalidation
- closeout log and full QA

## Stop Conditions

- evidence supports multiple materially different public branch contracts
- operator product preference is required to choose promotion
- the selected outcome contradicts Loophole version or recovery authority
- linear artifacts regress
- full QA fails

## Next Task

Return to the g01 generation runway. Start g01.012 characterization. The
optional tree implementation remains planned after g01.016.
