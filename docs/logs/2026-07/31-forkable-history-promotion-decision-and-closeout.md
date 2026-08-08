# Forkable History Promotion Decision And Closeout

Date: 2026-07-31
Card: 069
Roadmap: g01.011

## Result

Decision: `Promote`.

Promote the proven fork-tree semantics into planned g01.017 as a separate
optional layer. Retain the Card 068 workspace as private executable research.
Do not publish it, add it to the root workspace, or enable branch mode during
the first Loophole migration.

The public history system remains linear. Card 069 closes g01.011 without
claiming production branch artifacts.

## Gate Decision

| Gate | Evidence | Decision |
| --- | --- | --- |
| topology | immutable single-parent nodes preserve divergent futures | promote |
| identity | injected branch refs remain stable as heads advance | promote |
| navigation | deterministic preferred redo and one atomic LCA checkout pass | promote |
| failures | stale, apply-failure, and verified rollback cases preserve graph authority | promote |
| retention | current, named, and pinned lineages remain protected under bounded pruning | promote |
| checkpoints | bounded opaque consumer refs work without product snapshot authority | promote |
| persistence semantics | strict validation and independent migration pass | promote |
| persistence representation | JSON numeric byte arrays expand payloads excessively | replace |
| ordinary projection | one bounded linear path remains credible | promote |
| alternate projection | eager derived paths duplicate too much lineage | replace |
| public artifacts | no branch API or dependency entered the released linear graph | retain |

## Performance And Storage

The document shape reaches depth 128 with four alternates, 132 nodes, and a
99,295-byte prototype envelope. The Loophole shape reaches depth 2,048 with 64
alternates, 2,112 nodes, 540,672 retained payload bytes, and a 1,025-step LCA
route. Its measured plan and apply times are 650 microseconds and 818
microseconds.

The topology cost is credible. Two representations are not:

- eager derived paths expand 2,112 nodes into 67,648 projected entry refs
- JSON numeric byte arrays expand 540,672 payload bytes into a 7,534,856-byte
  envelope

g01.017 therefore requires lazy or paged alternate projections and a dense
payload representation. These are production gates, not optional tuning.

## Loophole Analysis

Forkable history can preserve alternate editing futures, stable named lines,
and checkpoint-relative navigation without replacing Loophole's project
versions. It can reuse the typed mutation, inverse, atomic transaction,
snapshot, journal, and recovery seams already proven by linear history.

The migration risk is larger than the semantic benefit needed for the first
consumer cutover. Loophole has no live branch implementation to preserve, and
combining linear adoption with new graph persistence would make rollback and
durability evidence harder to isolate.

Decision:

- g01.015 adopts the linear system with branch mode disabled
- g01.016 proves a real linear consumer and release checkpoint
- g01.017 implements the optional tree layer afterward
- Loophole tree adoption, if wanted, is a separate measured migration

## Package Boundary

The planned working names are:

- `longhorn-history-tree`
- `longhorn-tauri-history-tree`
- `@inflatable-cookie/longhorn-history-tree`

No package exists yet. The dependency direction is downward only: the tree
layer may depend on `longhorn-history`; the linear crate and minimal clients
must never depend on the tree layer. Renderer and Poodle surfaces expose only
bounded metadata and navigation controls, never product payloads.

## Disposition

Retained:

- all public linear history behavior and artifacts
- immutable nodes with one payload owner
- stable first-class branch references
- deterministic preferred redo
- atomic LCA checkout through the existing transaction seam
- protected count and encoded-weight retention
- bounded opaque checkpoint references
- strict structural validation and independent migration

Changed:

- fork semantics move from private research to a planned optional production
  lane
- alternate projections become lazy or paged production requirements
- production persistence must use a dense payload representation

Rejected:

- direct publication of the prototype
- branch mode as the default or as part of the first Loophole migration
- derived paths as branch identity
- eager unbounded path projection
- numeric JSON byte arrays for production payload storage
- unifying history with project versions, variants, merge, collaboration, or
  event sourcing

Deferred:

- production graph, persistence, protocol, Tauri, Svelte, and Poodle packages
- tree-enabled consumer artifact proof
- any Loophole branch-mode adoption

## Canonical Promotion

The decision updates contract 008, architecture, package topology, system
inventory, shared-system spec, Loophole migration constraints, g01 front
doors, and research memo 015. Roadmap g01.017 and Cards 070-074 capture the
production runway.

Standing bare-`continue` authority is recorded in AGENTS.md and contract 001.
It replaces repeated per-card authorization prompts without weakening stop
conditions or authorizing destructive external actions.

## Artifact Boundary

Card 068 proved that the public artifacts remained byte-identical to Card 067:

| Artifact | SHA-256 |
| --- | --- |
| `longhorn-history` Rust inventory | `acbe9977915042a59ab746bb999ef457c004badc5b1e9da323e1d645a3497292` |
| `@inflatable-cookie/longhorn-history` tarball | `a73813a2c43bdcb45934c85bbe15c447db5af92e8abb1a3cc010dfb0aae73333` |
| Poodle artifact set | `39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74` |

Card 069 re-runs the prototype selector and linear artifact proof. No branch
artifact becomes public through this decision.

## Validation

- `effigy qa:history-tree-prototype` passed: 13 semantic tests
- `effigy qa:northstar:g01-history-decision` passed
- `effigy proof:history-system-artifacts` passed with unchanged recorded hashes
- full `effigy qa` passed

## Next Task

Start g01.012 characterization. g01.017 waits behind the g01.016 linear
consumer and release checkpoint.
