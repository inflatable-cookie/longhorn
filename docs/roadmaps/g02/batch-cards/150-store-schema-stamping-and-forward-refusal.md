# 150 Store Schema Stamping And Forward Refusal

Status: ready
Owner: Tom
Roadmap: g02.009 batch 1
Governing refs: contracts 018 and 004; research memo 019
Depends on: none
Auto-start next card: no

## Objective

Record the schema version that wrote every persistent store, and refuse to
load a store written by a newer schema than the reader understands.

## Rationale

All channels ship under one bundle identity, so a nightly build and a
production build share these stores. Every nightly install eventually
rejoins production — automatically, once production reaches the same
version. Without a stamp, the production reader parses a newer store
best-effort, drops the fields it does not recognize, and writes the result
back. The data is gone and nothing reported it.

This is why the card gates the milestone: a channel that can write an
unstamped store is not shippable.

## Scope

- `longhorn-config`, `longhorn-settings`, `longhorn-history`,
  `longhorn-history-tree` persistence paths
- one shared store-version contract across all four; not four conventions
- backup and restore archives, which carry stores across time as well as
  across channels

## Steps

1. Define the shared store-version shape in `longhorn-core` alongside the
   existing diagnostics seam. One type, one comparison rule.
2. Stamp on write in all four stores.
3. Refuse on read when the recorded version exceeds the reader's, with a
   typed error naming both versions. Never partial-parse, never write back
   a store that failed to fully load.
4. Decide and record the treatment of existing unstamped stores — they
   predate the stamp and must remain loadable. A missing stamp is not a
   newer stamp.
5. Extend the same rule to backup archive admission.
6. Tests: newer-schema refusal per store, equal-version load, unstamped
   legacy load, and a round-trip that proves a refused load leaves the file
   untouched.

## Acceptance Criteria

- all four stores stamp and check under one shared contract
- a newer-schema store is refused with a typed error and left unmodified
- unstamped legacy stores still load
- backup restore honours the same rule
- workspace QA passes; no crate or package count change

## Evidence Required

- the shared version contract and its comparison rule
- per-store refusal tests and the untouched-file proof
- the recorded decision on legacy unstamped stores

## Stop Conditions

- a store's on-disk format cannot carry a stamp without breaking existing
  readers in consumer applications

## Next Task

Card 151, once consumer coordination for the new crates is agreed.
