# Private Forkable History Tree Prototype

Date: 2026-07-30
Card: 068
Roadmap: g01.011

## Result

The private nested Rust workspace proves immutable single-parent topology,
stable branch references, divergent record, deterministic preferred redo,
atomic LCA checkout, protected pruning, opaque checkpoints, strict
persistence, independent migrations, and linear-default projection.

The prototype remains non-publishable and outside the root workspace. No
consumer repository changed. The public linear package remains the only
compatibility claim.

## Topology And Branch Evidence

| Question | Result |
| --- | --- |
| divergent record | former and new futures remain as separate child nodes |
| payload authority | one immutable payload per node; branch refs contain no payload |
| branch identity | stable injected id survives head advance |
| branch metadata | bounded optional name and annotation plus pin outside nodes |
| preferred redo | deterministic child changes on record and successful checkout |
| derived paths | useful read model, unstable identity, expensive at width |
| first-class refs | required for stable naming, pinning, selection, and retention |

Derived root-to-leaf paths are not a replacement for branch references. They
duplicate lineage in projections and should remain lazy, bounded, and opt-in
if promoted.

## Navigation And Failure Matrix

| Case | Product model | Graph |
| --- | --- | --- |
| successful branch checkout | complete mixed undo/redo route | commits source-to-target position once |
| stale plan | untouched; transaction not called | untouched |
| apply failure, verified rollback | exact source | exact source |
| rollback failure | terminal partial-model evidence | exact source |

Checkout computes one lowest common ancestor and reuses the public typed
undo/redo steps and rollback evidence. The mixed plan and transaction remain
private.

## Retention, Checkpoint, And Projection Evidence

- count and exact encoded-weight limits prune the oldest unprotected leaves
- current, named, and pinned branch lineages are protected
- impossible protected budgets terminate with exact graph invariance
- dangling anonymous branch and checkpoint references are reported on prune
- checkpoints store only bounded opaque consumer references
- nearest-ancestor replay accounting reports remaining entry count and weight
- the default read model is one past/current/future path
- branch and derived-path metadata are explicit opt-in projections

## Persistence Evidence

- strict `longhorn.private.history-tree` version 1 envelope
- complete graph, branch, preferred-child, checkpoint, and current-position
  validation before a graph is returned
- unknown fields, foreign authority/family, future versions, corrupt parents,
  invalid sequences, and dangling refs reject visibly
- structural and payload versions migrate independently by exact one-step
  hooks
- failed loads return no replacement authority
- current encode/load/encode is deterministic

The public payload migration target cannot be constructed outside
`longhorn-history`. The private prototype therefore mirrors that contract
without changing the frozen public package. Card 069 must decide whether a
public graph implementation should expose a safe constructor or own graph
persistence inside the existing crate.

## Measurements

Release build on the local arm64 development machine:

| Metric | Document | Loophole-shaped |
| --- | ---: | ---: |
| target depth | 128 | 2,048 |
| alternate paths | 4 | 64 |
| retained nodes / branch refs | 132 / 5 | 2,112 / 65 |
| payload bytes per node | 32 | 256 |
| retained payload bytes | 4,224 | 540,672 |
| payload allocations | 132 | 2,112 |
| LCA checkout steps | 65 | 1,025 |
| plan / apply | 24 µs / 40 µs | 650 µs / 818 µs |
| derived projection refs | 388 | 67,648 |
| derived projection | 102 µs | 14,662 µs |
| encoded JSON envelope | 99,295 B | 7,534,856 B |
| encode / decode | 138 µs / 423 µs | 7,052 µs / 15,436 µs |
| pruned leaves / prune | 4 / 109 µs | 64 / 13,953 µs |

Durations are observations, not acceptance thresholds. Exact counts and sizes
are repeatable through `effigy measure:history-tree-prototype`.

The Loophole-shaped result exposes two promotion constraints:

- eager derived paths expand 2,112 nodes into 67,648 projected entry refs
- JSON numeric byte arrays expand 540,672 payload bytes into a 7.53 MB
  envelope

A promoted design should use bounded/lazy alternate projections and a denser
payload representation. The topology and atomic checkout costs are credible
at the measured shape.

## Boundary Audit

The public artifact proof remains byte-identical to Card 067:

| Artifact | SHA-256 |
| --- | --- |
| `longhorn-history` Rust inventory | `acbe9977915042a59ab746bb999ef457c004badc5b1e9da323e1d645a3497292` |
| `@inflatable-cookie/longhorn-history` tarball | `a73813a2c43bdcb45934c85bbe15c447db5af92e8abb1a3cc010dfb0aae73333` |
| Poodle artifact set | `39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74` |

No branch API, prototype dependency, Loophole payload, project-version
identity, collaboration, merge, or event-source claim entered a public
artifact.

## Validation

- `effigy qa:history-tree-prototype`
- `effigy measure:history-tree-prototype`
- `effigy proof:history-system-artifacts`
- full `effigy qa`

Rust 1.85 compiles and tests the nested workspace. Formatting and Clippy use
the installed current toolchain because those 1.85 components are not
installed.

## Checkpoint

Card 068 completes the evidence prototype. Card 069 closed the decision gate
on 2026-07-31: promote the proven semantics into a later optional production
lane and retain this workspace as private research.
