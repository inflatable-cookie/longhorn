# Fork-tree Identity, Topology, And Branches

Date: 2026-08-03
Card: 070
Roadmap: g01.017

## Result

`longhorn-history-tree` is now an optional pure Rust production package. It
owns:

- bounded injected branch ids and mutable branch metadata
- immutable single-parent typed nodes
- canonical insertion-ordered child indexes
- stable branch refs with mutable heads and no payload
- current branch and current node
- complete-state validation before authority admission
- lossless divergent record and exact receipts

The package depends only on `longhorn-core` and `longhorn-history`.
`longhorn-history` and all linear artifacts retain no upward tree dependency.

## Prototype Translation

Retained:

- lowercase stable id grammar and 128-byte ceiling
- 256-byte name and 4 KiB annotation ceilings
- one payload owner per immutable node
- stable branch identity outside derived paths
- former-future preservation on divergent record
- deterministic preferred child on committed record
- sequence-ordered derived child indexes

Rejected or deferred:

- direct promotion of private prototype source
- persistence serde and expanded numeric byte arrays
- navigation, pruning, checkpoints, and projections
- prototype-only test shortcuts and unchecked construction
- mutation before every fallible record check completes

Production import rejects duplicate ids, sequences, and commit revisions;
missing or backwards parents; invalid heads, positions, and preferred edges;
invalid next sequence; hard-limit violations; and weight overflow. Child indexes
are always derived from validated nodes.

## Evidence

Document fixtures prove bounded identity, typed payload ownership, stable refs,
exact receipts, failure invariance, and both preserved futures. A
Loophole-shaped 97-node Pulse fixture forks behind the live head and retains
both typed futures. The invariant matrix rejects duplicate identity, sequence,
revision, branch, and preferred-parent evidence; missing and backwards
parents; invalid heads and positions; invalid next sequence; missing branches;
and encoded-weight violations.

Focused Effigy QA covers formatting, Clippy, Rust tests, package contents, and
the downward-only dependency graph. The existing minimal and Loophole-shaped
linear artifact proof also passes unchanged and contains no tree package.

## Boundaries

Loophole remains unchanged with branch mode disabled. Product mutation,
project versions, snapshots, journals, recovery, merge, and collaboration stay
consumer authority. Card 070 makes no release or compatibility claim.

## Next Task

Execute Card 071. Add atomic LCA navigation, protected retention, and opaque
checkpoint accounting over the production graph.
