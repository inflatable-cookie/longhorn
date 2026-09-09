# g02.011 IPC Boundary Validation

Status: complete — 2026-08-12
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

Eleventh g02 task. Opened by measurement rather than by a research
memo: the audit's P2-10 finding named a line count, and the inventory in
Card 160 turned that into a coverage table showing the layer is not
uniformly implemented at all.

## Execution Plan

### Stage 1. Inventory and derivation

- [x] Card 160 (complete; closeout recorded below)
  records the measured inventory, emits bound constants from the authority,
  and derives the structural validators

## Goals

- [x] every package rejects unknown and missing fields at the boundary
- [x] every wire-visible Rust bound is enforced from a generated constant
- [x] no hand-copied magic numbers remain at the boundary
- [x] the one genuinely semantic rule is hand-owned and labelled as such

## Acceptance Criteria

- [x] the inventory re-runs with uniform columns
- [x] `check:bindings` fails on a deliberate Rust-side bound change
- [x] the 12 client modules and 187 call sites are unchanged

## Explicit Non-goals

- version negotiation. No package supports two protocol versions today, and
  this task does not add that; it removes the misleading name.
- validation in `svelte` and `poodle`, which sit downstream of the boundary
  and correctly carry none.

## Progress

Steps 2 and 3 have landed on bridge. Bounds come from the Rust authority, and
so does the connection state/reason matrix — which this task's card had
recorded as the one rule that could not be derived. It could; the rule lived in
a `matches!` arm rather than a type, and `ts-rs` carrying only types was
mistaken for the rule being underivable.

Step 1 is decided and recorded in contract 010: the boundary matches the Rust
authority's strictness and derives it. The measurement that settled it —
`deny_unknown_fields` on 332 Rust types against nine TypeScript packages that
accept them — showed the boundary had been asymmetric by accident rather than
by choice.

Steps 4 through 6 are now unblocked and mechanical: emit structural validators,
migrate package by package deleting each hand-written original in the same
commit, and rename the surface.

## Next Task

Steps 1-5 are done. Every package that owns a boundary rejects an unknown and
a missing field, at its entry points and below them, and all 175 key checks
derive from the Rust structs rather than from hand-written lists. The client
modules are unchanged throughout.

`layout` left scope: it has no client and receives no IPC payload, so a field
map there would build a boundary the package does not own.

Tagged unions stay on the lenient path until a per-variant field map exists —
their allowed keys depend on the discriminant, so one flat list is wrong.

Step 6 is done. The modules are `validation.ts` and `validation/`, and the
symbols are `assertValidX` and `XProtocolValidationError`. Nucleus was
migrated in the same pass; Loophole needed no source change.

The task's work is complete.

Closed 2026-08-12. The header said `ready` for a week after the body said this,
which is the kind of drift the front doors exist to prevent.

The tagged-union deferral above is now its own task, g02.018. It stopped
being theoretical when Cards 183 to 186 needed three hand-written per-variant
key lists, and one union that already had one was wrong.

## Absorbed records

Absorbed from `batch-cards/` in the flattened-task migration; the directory is removed and git history is the full-fidelity archive.

- Card 160: complete — IPC validation derived from the authority. The card's own status line still reads in progress; its steps 1-6 are recorded landed in this task's Progress and Next Task sections and the task closed 2026-08-12. The card line is stale and superseded by that closeout.
