# g02.018 Tagged-Union Boundary Validation

Status: in progress — batch 1 complete 2026-08-12
Owner: Tom
Governing refs: contract 010; contract 011; contract 012
Depends on: g02.011 (complete)

## Outcome

Every boundary type validates against a field map generated from the Rust
authority — except tagged unions, which are 181 of them across all twelve
domains. Those stay on a lenient path, and where a consumer needed strictness
it was written by hand.

This milestone generates the per-variant map and adopts it, so the answer to
"which keys does this variant allow" comes from the enum rather than from
somebody's memory of it.

## Why This Stopped Being Theoretical

g02.011 deferred it deliberately and said why:

> Tagged unions stay on the lenient path until a per-variant field map exists —
> their allowed keys depend on the discriminant, so one flat list is wrong.

That reasoning was right. The consequence arrived with Cards 183 to 186, which
needed strict per-variant checks on three unions and had to hand-write all
three: `ForkPathTargetProjection`, `ForkNavigationTargetProjection` and
`ForkPruneResult`.

One of them was already wrong before that. `assertForkNavigationCommand`
allowed exactly `["kind"]` for every target but `checkout`, so Card 181's
`checkoutBranchRoot` — which carries a `branchId` — would have been rejected at
the boundary the first time a consumer sent one. It shipped, and nothing
caught it, because the list was hand-written and no generated fact contradicted
it.

That is the whole argument. A hand-written key list is a second copy of the
enum, and the second copy drifts silently.

## What Exists

`crates/longhorn-bindings/src/generation.rs`:

- `field_map` renders `Record<string, readonly string[]>` for plain objects and
  **returns tagged unions as `skipped`**. Every domain prints them as a warning
  on every generate, and nothing acts on the warning.
- `tagged_variants` already splits a union by discriminant — but returns only
  the discriminant values, which is what the `*_TARGETS` and `*_STATUSES`
  constants are built from.
- `plain_object` refuses anything with more than one brace group, on purpose,
  with a comment explaining that a naive balance check produces nonsense field
  names from a union.

So the parser already knows how to find a union and how to read a flat field
list. What is missing is splitting a union into its arms and applying the
second to each.

## Scope

The generator, then adoption. Adoption is per domain and most domains are on
the lenient path today, so emitting the map changes nothing for them until
someone uses it. Batch 1 proves the generator against the domain that already
has three hand-written maps to delete.

## Execution Plan

- [x] **Batch 1. Generate it, and delete the hand-written three** (Card 187,
      complete 2026-08-12).
      A per-variant map beside the flat one, and `history-tree` adopting it in
      place of `PATH_TARGET_FIELDS`, `NAVIGATION_TARGET_FIELDS` and the inline
      `ForkPruneResult` keys.
- [ ] **Batch 2. Adopt across the remaining domains.** Eleven domains, mostly
      mechanical, and worth doing only once batch 1 has shown the map is right.
      Some domains validate no unions at all today; those gain strictness they
      never had, which is a behaviour change per domain and wants its own
      evidence.
- [ ] **Batch 3. Make the warning an error.** Once nothing is skipped, a union
      the generator cannot parse should fail the build rather than print. Not
      before, or every generate fails on the domains batch 2 has not reached.

## Goals

- [ ] No hand-written per-variant key list survives in any `validation.ts`.
- [ ] A variant that gains a field is rejected at the boundary until the
      bindings are regenerated, the same as a plain object today.
- [ ] The generator's "not in the field map" warning goes to zero.

## Acceptance Criteria

- [ ] The per-variant map is generated for all 181 unions, including generic
      ones and unit variants that carry only the discriminant.
- [ ] `history-tree` validates all three of its unions from the generated map,
      and the `checkoutBranchRoot` case that was wrong has a test.
- [ ] `effigy qa` passes, including `check:bindings`.

## Explicit Non-goals

- No runtime type checking. The map holds names, as the flat one does. A field
  that keeps its name and changes type is not this milestone's problem.
- No change to the lenient path's behaviour in domains batch 2 has not reached.
  Emitting a map nobody reads is inert, and that is the point of splitting it.

## Next Task

Batch 2. Card 187 showed the generator side costs one function and two
helpers, and that adoption in a domain costs three call sites and a lookup
helper. Eleven domains at that size is one card, not eleven — but most of them
validate no union at all today, so each gains strictness it never had, and that
is the part worth sizing per domain rather than in aggregate.

## Planning Checkpoint

After batch 1. Whether batch 2 is one card or eleven depends on how much each
domain's adoption actually costs, and one worked example answers that better
than an estimate.
