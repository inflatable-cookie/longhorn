# 074 Fork-tree Artifact Proof And Closeout

Status: complete
Owner: Tom
Roadmap: g01.017 batch 4
Governing refs: contracts 003, 008, 012, and 013; Cards 067 and 070-073
Depends on: Card 073
Auto-start next card: no
Completed: 2026-08-03

## Objective

Prove the optional fork-tree layer through isolated linear-only and
tree-enabled artifacts, then decide whether any consumer adoption lane opens.

## Scope

- produced Rust and TypeScript artifacts
- minimal linear-only dependency graph
- document tree-enabled graph
- Loophole-shaped graph and navigation trace
- persistence, migration, pruning, checkpoint, and projection failures
- size and performance comparison with Card 068
- capability, payload, authority, peer, and package audits
- composition and adoption guidance

## Out Of Scope

- silent Loophole branch enablement
- project versions, collaboration, or merge
- registry publication without release authority

## Steps

1. Pack all linear and tree artifacts.
2. Install isolated linear-only and tree-enabled consumers.
3. Run equal native and renderer semantic traces.
4. Re-run the full failure and persistence matrix.
5. Compare measured depth, width, size, and projection costs.
6. Audit optional-edge absence and product authority.
7. Publish composition and later adoption guidance.
8. Run full Effigy QA and close g01.017.

## Acceptance Criteria

- linear-only artifacts remain byte and dependency isolated
- tree consumers resolve no sibling source
- graph behavior matches the promoted contract
- dense persistence and bounded projections meet Card 072-073 gates
- no product payload or apply authority enters shared clients
- full Effigy QA passes

## Evidence Required

- artifact identities and clean installs
- semantic and failure traces
- size/performance report
- dependency, payload, capability, and authority audits
- closeout log and full QA

## Stop Conditions

- linear artifacts regress
- clean installs resolve prototype or sibling source
- measured costs exceed the recorded production bounds
- full QA fails

## Next Task

No tree consumer lane auto-starts. Choose one separately from evidence; do not
enable branch mode implicitly. Nucleus grouped restore may resume in its own
repository because g01.019 already closed that Longhorn blocker.
