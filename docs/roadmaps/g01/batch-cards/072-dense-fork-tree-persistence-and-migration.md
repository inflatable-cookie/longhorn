# 072 Dense Fork-tree Persistence And Migration

Status: ready
Owner: Tom
Roadmap: g01.017 batch 2
Governing refs: contracts 004, 008, and 012; Cards 065, 068, and 071
Depends on: Card 071
Auto-start next card: yes

## Objective

Implement a strict graph envelope with dense payload bytes, independent
structural and payload migration, and complete pre-admission validation.

## Scope

- separate stable graph format family
- dense deterministic payload representation
- exact structural and payload codec versions
- one-step migration hooks
- full topology, ref, sequence, revision, weight, and current-position checks
- corruption, foreign, future, truncated, and oversized input
- deterministic encode/load/encode
- measured document and Loophole-shaped size

## Out Of Scope

- choosing app storage paths or durability cadence
- checkpoint data
- journal, snapshot, autosave, and recovery policy
- renderer transport

## Steps

1. Freeze the dense wire representation and limits.
2. Resolve the shared payload-migration target construction seam.
3. Encode complete graph authority deterministically.
4. Validate and migrate before returning replacement state.
5. Add malicious and corrupt input fixtures.
6. Compare size and load cost with Card 068.

## Acceptance Criteria

- payload bytes do not serialize as JSON numeric arrays
- future and corrupt input reject visibly
- failed load returns no replacement authority
- structure and payload migrate independently
- exact graph round-trips deterministically
- Loophole-shaped envelope size is materially denser than Card 068

## Evidence Required

- version and migration matrix
- topology corruption matrix
- deterministic fixture
- encoded-size and allocation report
- storage-authority audit

## Stop Conditions

- dense representation is platform-dependent
- migration needs consumer payload meaning in Longhorn
- load must mutate live authority before validation completes

## Next Task

Card 073 adds bounded metadata clients and optional UI composition.
