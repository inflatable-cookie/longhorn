# 070 Fork-tree Identity, Topology, And Branches

Status: planned
Owner: Tom
Roadmap: g01.017 batch 1
Governing refs: contracts 001, 008, and 012; Cards 068-069
Depends on: g01.016 checkpoint
Auto-start next card: yes

## Objective

Implement a publishable optional pure Rust graph authority with immutable
single-parent nodes and stable first-class branch references.

## Scope

- optional `longhorn-history-tree` package boundary
- bounded branch identity and metadata
- immutable nodes and canonical child indexes
- current branch and current node
- divergent record preserving the former future
- deterministic validation and exact receipts
- document and Loophole-shaped fixtures

## Out Of Scope

- navigation execution
- pruning and checkpoints
- persistence and clients
- Loophole repository writes

## Steps

1. Revalidate package and compatibility baselines.
2. Promote prototype identities and immutable topology without copying test
   shortcuts.
3. Implement stable branch refs and divergent record.
4. Validate all indexes, refs, sequences, and revisions.
5. Prove linear-only dependency absence.
6. Record exact retained and rejected prototype behavior.

## Acceptance Criteria

- each node owns one typed payload
- branch refs own no payload and survive head advance
- divergent record preserves both futures
- malformed topology cannot enter authority
- linear crates and artifacts do not depend on the tree package
- both fixture shapes pass

## Evidence Required

- topology and identity fixtures
- divergence receipts
- invariant rejection matrix
- dependency audit
- focused Rust QA

## Stop Conditions

- stable branch identity needs project-version identity
- linear history must depend upward on tree state
- one payload needs multiple node owners

## Next Task

Card 071 adds atomic navigation, retention, and checkpoints.
