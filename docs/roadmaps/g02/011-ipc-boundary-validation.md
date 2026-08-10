# g02.011 IPC Boundary Validation

Status: ready
Owner: Tom
Updated: 2026-08-08
Governing refs: contracts 010 and 012; the P2-10 audit finding
Depends on: none

## Outcome

The TypeScript IPC boundary validates uniformly, from the Rust authority
that already defines the shapes and the bounds, instead of from 5,330
hand-written lines whose coverage nobody could describe until it was
measured.

## Generation Runway

Eleventh g02 milestone. Opened by measurement rather than by a research
memo: the audit's P2-10 finding named a line count, and the inventory in
Card 160 turned that into a coverage table showing the layer is not
uniformly implemented at all.

## Execution Plan

### Batch 1. Inventory and derivation

- [ ] [Card 160](batch-cards/160-ipc-validation-derived-from-authority.md)
  records the measured inventory, emits bound constants from the authority,
  and derives the structural validators

## Goals

- [ ] every package rejects unknown and missing fields at the boundary
- [ ] every wire-visible Rust bound is enforced from a generated constant
- [ ] no hand-copied magic numbers remain at the boundary
- [ ] the one genuinely semantic rule is hand-owned and labelled as such

## Acceptance Criteria

- [ ] the inventory re-runs with uniform columns
- [ ] `check:bindings` fails on a deliberate Rust-side bound change
- [ ] the 12 client modules and 187 call sites are unchanged

## Explicit Non-goals

- version negotiation. No package supports two protocol versions today, and
  this milestone does not add that; it removes the misleading name.
- validation in `svelte` and `poodle`, which sit downstream of the boundary
  and correctly carry none.

## Progress

Steps 2 and 3 have landed on bridge. Bounds come from the Rust authority, and
so does the connection state/reason matrix — which this milestone's card had
recorded as the one rule that could not be derived. It could; the rule lived in
a `matches!` arm rather than a type, and `ts-rs` carrying only types was
mistaken for the rule being underivable.

Step 1 — deciding the target and recording it in contract 010 — still gates
steps 4 through 6, and is an operator decision: nine packages would gain key
validation and five would gain bounds, so there is no "current behaviour" to
preserve.

## Next Task

Card 160 step 2 is separable and worth taking first: emitting the bound
constants closes the only finding with a live drift mechanism, and does not
depend on agreeing the wider target.
