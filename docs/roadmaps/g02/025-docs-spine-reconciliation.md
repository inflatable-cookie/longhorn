# g02.025 Docs Spine Reconciliation

Status: complete
Completed: 2026-08-15
Owner: Tom
Updated: 2026-08-14
Governing refs: memo 023; contract 001; contract 002; contract 012
Depends on: none

## Outcome

The handwritten canonical docs describe the system that exists. Card 179's
absorption is reflected in architecture and contracts, front doors carry real
counts, the held-surface register earns its "single source of truth" claim,
and a consumer following the guides reaches a working install.

## Generation Runway

Memo 023's critical docs finding: the generated `api-surface.md` is the most
accurate doc in the repo, and the handwritten spine is the drift. Three docs
still specify the layout-container model deleted 2026-08-10; contract 002
contradicts its own absorbed section; the generation index contradicts itself
on publication. Reference quality means the spine is evidence, not folklore —
this milestone is the sweep.

## Planning Gaps

- **Card 149's receipt regeneration stays operator-held.** Card 217 documents
  the post-consolidation install path against the *current* tree and names the
  frozen receipt as historical; it does not unfreeze the receipt.
- **The card127 receipt cannot be honestly regenerated** (its hash chain locks
  the error in). Card 216 annotates the discrepancy in
  `docs/reference/private-0-1-candidate.md`; rewriting history is rejected.

## Execution Plan

### Batch 1. Architecture and contracts

- [x] [Card 215](batch-cards/215-post-179-architecture-sweep.md): rewrite
  `system-architecture.md`'s hosting/layout sections against the
  Surface-as-layout model; revise contract 002's pre-absorption body; break
  the 002↔014 pointer loop; resolve the "no Surface package" acceptance
  criterion against `nucleus-no-surface-proof`; correct
  `package-topology.md`, `system-inventory.md`, and the stale test comment.
  **Landed 2026-08-15** — contract 002 reads as one contract, the topology
  table diffs equal to the workspace, and the proof keeps its name with the
  contract recording what it actually asserts. Card 211's empty divergence
  list meant restating sufficed.

### Batch 2. Registers and indexes

- [x] [Card 216](batch-cards/216-front-door-truth-and-register-freshness.md):
  single-source or delete the handwritten package/crate counts; held-surface
  rows corrected and a freshness gate added to `verify-held-surface.ts`;
  generation index reduced to one next-task pointer; contract-index statuses
  and contract 020's internal contradiction fixed; publication-status
  amendment across contract 012 and the compatibility guide; card127 receipt
  annotated. **Landed 2026-08-15** — the gate caught both stale rows on its
  first run; the index has one pointer; the receipt is annotated, not
  rewritten.

### Batch 3. Guides

- [x] [Card 217](batch-cards/217-guide-repair.md): guide repair, verified in
  a scratch consumer. **Landed 2026-08-15** — installable manifest, frozen
  receipt named as history, Surface-as-layout throughout, and the four
  stragglers the 215 sweep missed caught by review.

## Dependency Shape

```text
memo 023 (C2, H5, H6, M1-M5 docs findings)
 └─ 025 docs spine reconciliation
     ├─ 215 architecture + contracts   (first — cards 216/217 cite it)
     ├─ 216 registers + indexes        (needs 215's contract state)
     └─ 217 guides                     (needs 215's model language)
```

215 first: the register and guide fixes quote the architecture, so the
architecture stops being wrong before anything points at it.

## Goals

- [x] no canonical doc names a deleted crate as load-bearing
- [x] contract 002 reads as one contract, not two drafts stapled together
- [x] every count in a front door is generated or gone
- [x] the guides' copy-paste paths work against the tree they ship with

## Acceptance Criteria

- [x] `LayoutContainerId` appears in live docs only as history
- [x] `verify-held-surface.ts` flags rows whose named trigger cards are closed
- [x] `generation-index.md` carries exactly one live next-task pointer and no
  self-contradictory publication claims
- [x] a fresh reader following `getting-started.md` + `package-selection.md`
  reaches a resolvable manifest

## Explicit Non-goals

- Rewriting handoffs, logs, or historical research memos. The honest-history
  convention stands; drift findings there are records, not errors.
- Regenerating or editing frozen receipts (card127, Card 149).

## Next Task

Milestone complete. g02.027 (Cards 221-223) remains in the audit suite, plus
the blocked 210 and held 218.

## Planning Checkpoint

After Batch 1. If revising contract 002's body surfaces behavior the absorbed
sections never specified (the Card 211 port may find some), that feeds back
here as a contract amendment, not prose.
