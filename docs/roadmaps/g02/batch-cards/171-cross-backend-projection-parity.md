# Card 171 Cross-backend Projection Parity

Status: complete
Completed: 2026-08-09
Owner: Tom
Updated: 2026-08-09
Governing refs: memo 022; contract 013; contract 020
Depends on: Card 169; Card 170

## Why

Card 169 closed on an admission and memo 022 repeated it: nobody has put the
Svelte and GPUI renderings of the same facts side by side, so "the two
backends agree" is an assumption. Memo 022's agreements — severity tones,
operation labels and tones, cancel eligibility, compatibility labels — were
established by *reading both implementations*, which catches what is true
today and nothing about tomorrow.

Card 170 removed the drift risk for wording by generating it. Everything else
in a projection is still stated twice: which tone a severity maps to, whether
a state can be cancelled, how progress becomes a bar. Those are the parts a
reader has to check by eye, and they are exactly the parts that were written
independently and happen to agree.

## Approach

**A shared fixture that both sides check against, not a generated one.**

`fixtures/parity/projection-v1.json` states the cross-backend contract as
data: for each input fact, the user-visible values a surface must produce. A
Rust test asserts `longhorn-poodle` matches it. A TypeScript test asserts
`longhorn-poodle-svelte` matches it. Either side drifting fails its own gate,
and the fixture is the thing a person argues with.

Generation was considered and rejected. Generating from Rust would make Rust
right by construction and reduce the TypeScript side to a mirror — which is
correct for *wording* (Card 170) and wrong for *behaviour*, where the point is
that two independent implementations agree. A fixture nobody generates is a
statement both sides answer to.

It also avoids a mechanical problem: tone mapping lives in `longhorn-poodle`,
which depends on `poodle-specs`, and `longhorn-bindings` must not.

## Scope

Only what both tiers actually project. Where one tier has no counterpart the
fixture says so rather than inventing one:

- notification severity to tone
- operation state to tone and label
- operation progress to a bar
- operation cancel eligibility
- restore compatibility to a label and to selectability

## Result

**Thirty-nine cases, both sides green first try.** Every agreement memo 022
established by reading held under test: five severity tones, seven operation
states with tone and label, three progress shapes, nine cancel-eligibility
combinations, thirteen restore classifications with label and selectability.

That is the useful outcome. The two tiers were written independently, months
apart, in different languages, and they agree on all thirty-nine. What changes
is that they now *keep* agreeing, or a gate goes red.

### The harness can fail

A parity suite nobody has seen fail is theatre. Mutating two fixture values —
`cancelled` to danger, and the migration label — turned five Rust assertions
and two TypeScript tests red. Restored and re-verified green. Worth thirty
seconds to know the thing bites.

### Two deliberate differences, recorded rather than omitted

The fixture carries a `deliberateDifferences` list, and a test asserts each
entry has a stated reason. A parity suite that lists only agreements reads as
though there are no differences.

- The Rust toast prefixes a `Critical` title; the Svelte tier has no toast
  projector at all, so there is nothing to compare. Memo 022, D5.
- `notificationStatusLabel` appends `", unseen"`; Rust carries no read state
  into the toast. Memo 022, D7, still open.

## Do Not

- Compare pixels. The claim is that the two backends decide the same things,
  not that two renderers draw identically — they should not.
- Add a case to the fixture without a reason a person can read. A parity table
  full of unexplained rows is a table nobody will maintain.

## Acceptance Criteria

- [x] every agreement memo 022 asserted by reading is asserted by a test on
  both sides
- [x] a change to either tier's mapping fails a gate — proved by mutating the
  fixture and watching both sides go red
- [x] cases the two tiers deliberately answer differently are recorded as
  such, not omitted

## Stop Conditions

Did not fire. Nothing disagreed.
