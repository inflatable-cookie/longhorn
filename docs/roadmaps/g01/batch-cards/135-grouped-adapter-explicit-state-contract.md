# 135 Grouped Adapter Explicit-state Contract

Status: complete
Owner: Tom
Roadmap: g01.019 batch 5
Governing refs: contracts 001, 004, and 012; Nucleus contract 032 and g05.046
Depends on: Card 131
Auto-start next card: yes
Completed: 2026-08-03

## Objective

Replace grouped restore's digest/optional ambiguity with one explicit present
or absent semantic-state model.

## Scope

- archive source state in adapter inspection
- target and current preview evidence
- confirmation digest form
- plan and receipt projections
- apply and verify request evidence
- public Rust names and accessors

## Steps

1. Freeze one serializable explicit state-evidence type.
2. Supply verified archive presence to adapter inspection.
3. Bind explicit target and current evidence into confirmations and plans.
4. Carry expected state and apply kind into both apply and verify requests.
5. Expose target and rollback evidence in terminal receipt entries.

## Acceptance Criteria

- absence is not represented by `Option<Sha256Digest>` in grouped state
- target and rollback absence are independently observable
- archive state and preview target presence cannot contradict
- confirmation encoding distinguishes absence from every digest
- existing present-only adapter behavior remains semantically unchanged

## Evidence Required

- contract diff
- public compile fixture
- confirmation and projection tests
- focused Clippy and Rust tests

## Stop Conditions

- a sentinel digest or payload is required
- a separate-adapter behavior change becomes necessary
- consumer domain meaning must enter Longhorn

## Next Task

Card 136 implements durable absent apply, rollback, and recovery.

## Evidence

- contract 004 defines explicit absent/present state and canonical confirmation forms
- `BackupAdapterInspectRequest` carries verified archive source state
- plan, apply, verify, and receipt APIs expose exact target and rollback evidence
- the external integration baseline pins public method signatures and serialized shapes
- focused Rust tests and Clippy pass
