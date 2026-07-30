# 068 Private Forkable History Tree Prototype

Status: planned
Owner: Tom
Roadmap: g01.011 batch 3
Governing refs: contract 008; research memo 015
Depends on: Card 067
Auto-start next card: no

## Objective

Build a non-publishable forkable history prototype over the proven payload and
transaction seams. Measure divergent topology, checkout, retention,
checkpoint, migration, and projection choices without changing the public
linear contract.

## Scope

- immutable single-parent entry nodes
- current node and stable branch identity experiments
- derived paths versus first-class branch references
- divergent record after undo
- deterministic preferred redo child
- lowest-common-ancestor checkout plan
- named, annotated, and pinned branches
- count and encoded-weight pruning
- opaque checkpoint references and replay-cost accounting
- graph persistence and payload migration
- linear-default and alternate-path projections
- realistic Loophole-shaped and non-editor workloads

## Prototype Behavior

A divergent record preserves the former future as an alternate child. Checkout
plans inverses to the lowest common ancestor, then forward payloads to the
target. The existing atomic consumer transaction applies the whole route
before graph commit.

The prototype may compare models. It cannot export a stable package, alter
linear artifacts, or become a Loophole dependency.

## Out Of Scope

- public branch API
- Loophole repository migration
- collaboration or merge
- project versions and variants
- event sourcing
- conflict-free replicated data types

## Steps

1. Freeze prototype questions and benchmark fixtures.
2. Implement immutable node topology and divergent record.
3. Compare derived paths with first-class branch references.
4. Implement deterministic preferred redo and branch switching.
5. Plan LCA checkout through the existing transaction seam.
6. Add labels, annotations, pinning, and current-branch behavior.
7. Add count and weight pruning with protected lineage.
8. Add opaque checkpoints and replay-cost accounting.
9. Add graph envelope, corruption, and payload migration experiments.
10. Project one linear default plus optional alternate metadata.
11. Benchmark depth, width, payload weight, checkout, encode/decode, and prune.
12. Record pass, failure, and unresolved evidence for Card 069.

## Acceptance Criteria

- divergent edits preserve both futures without duplicating payload authority
- preferred redo is deterministic
- checkout uses one atomic LCA route and preserves failure invariance
- pinned/current lineage cannot prune
- pruning terminates under count and weight budgets
- checkpoints remain opaque consumer evidence
- future/corrupt graph and payload versions reject visibly
- default projection stays linear when branch UI is absent
- realistic donor-shaped payload weight and depth are measured
- public linear package and artifacts remain unchanged

## Evidence Required

- topology and branch-reference comparison
- divergent record and preferred-redo fixtures
- LCA route and failure matrix
- pruning, pinning, checkpoint, migration, and corruption matrix
- linear/alternate projection fixtures
- benchmark report and allocation/encoded-size evidence
- public-package diff and dependency audit

## Stop Conditions

- graph work changes the public linear contract
- branch identity requires project-version identity
- checkout cannot use the atomic apply seam
- pruning cannot protect current or pinned lineage
- benchmark evidence is too weak for a promotion choice

## Next Task

Card 069 is planned. Make the explicit promote, retain, or reject decision from
the prototype evidence.
