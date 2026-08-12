# 187 Generated Variant Field Maps

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.018 batch 1
Governing refs: contract 010; contract 011; contract 012
Depends on: g02.011 (complete)
Blocks: batches 2 and 3
Auto-start next card: no

## Why

Three hand-written per-variant key lists went into `history-tree`'s
`validation.ts` across Cards 183 to 186, and a fourth had been there since
Card 181 with a bug in it. See the milestone. The generator already parses
unions and already parses flat field lists; it has never combined them.

## Step 1 — Split a union into its arms

`crates/longhorn-bindings/src/generation.rs`.

- [x] `tagged_field_map(declaration, tag)` returns each variant's discriminant
      value paired with its allowed keys, the discriminant included. A caller
      hands the result to `exact()`, which compares every key, so leaving the
      tag out would reject every payload.
- [x] Reuse `strip_ts_comments` and `field_names`. The first already exists
      because `ts-rs` puts doc prose between fields; the second already handles
      both separators `ts-rs` emits. Neither needs a union-specific variant.
- [x] Split on `|` **at brace depth zero only**. A union arm can hold a nested
      object or a generic with its own `|`, and splitting naively cuts an arm
      in half. `plain_object` refuses multi-brace bodies for the same reason
      and says so; this is the case it was refusing.
- [x] A unit variant renders as `{ "kind": "undo" }` and yields exactly
      `["kind"]`. That is the case the old hand-written default guessed at, and
      guessed right for every target except `checkoutBranchRoot`.

## Step 2 — Emit it

- [x] A second constant beside the flat map:
      `Record<string, Record<string, readonly string[]>>`, keyed by type name
      then by discriminant value. Beside rather than merged: the flat map's
      values are `readonly string[]` and a merged shape would make every
      consumer narrow before using either.
- [x] Generic unions lose their parameters from the key, as `plain_object`
      already does for plain objects. Same reason: a key with angle brackets is
      not a usable property name.
- [x] The `skipped` list shrinks to unions the splitter genuinely cannot read.
      Print those, and print the count, so batch 3 knows what it is waiting on.

## Step 3 — Adopt in history-tree, and delete the hand-written maps

`packages/longhorn/src/history-tree/validation.ts`.

- [x] `assertForkPathCommand` reads the generated map instead of
      `PATH_TARGET_FIELDS`.
- [x] `assertForkNavigationCommand` reads it instead of
      `NAVIGATION_TARGET_FIELDS`.
- [x] `assertForkPruneResult` reads it instead of its inline `["status"]` and
      `["status", "receipt"]`.
- [x] Delete all three hand-written constants. Not left beside the generated
      map as a fallback — a fallback is the second copy this card exists to
      remove.
- [x] An unknown discriminant is already rejected by the `oneOf` above each
      of these. Keep that ordering: a missing entry in the generated map then
      means the generator failed, not that a consumer sent something odd.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] A generator unit test splits a union with a unit variant, a
      single-field variant and a multi-field variant, and asserts all three
      key lists including the discriminant.
- [x] A generator unit test asserts an arm containing a nested object is not
      split in half.
- [x] A validation test sends `checkoutBranchRoot` with its `branchId` and is
      accepted — the case the hand-written list rejected.
- [x] A validation test sends `checkoutBranchRoot` with an extra key and is
      rejected, so the strictness is real and not a widened default.
- [x] No `Record<string, readonly string[]>` literal remains in
      `history-tree`'s `validation.ts`.

## Evidence

- [x] The tests above, named in the batch log.
- [x] The generator's skipped count, before and after.

## Stop Conditions

- Stop if splitting at brace depth zero does not separate every arm cleanly for
  all 181 unions. A splitter that is right for most of them and wrong for a few
  is worse than the lenient path, because the wrong ones reject valid payloads
  at a boundary rather than failing at generation.
- Stop if any union's arms do not all share one discriminant key. `ts-rs`
  should guarantee it for `#[serde(tag = "...")]`, but an untagged or adjacent
  union in the set would need a different shape and that is a modelling
  decision, not this card's.

## Continuation

Batch 2 adopts the map across the remaining eleven domains, sized by what this
one costs.

## Outcome — 2026-08-12

All three steps landed. `effigy qa` exit 0.

`history-tree` emits two maps, not one: its targets are tagged `kind` and its
results `status`, and one pass cannot know which key a union uses without
reading it. Two passes, joined, and a union readable under neither is reported.
For this domain that report is empty.

The generated `checkoutBranchRoot` entry is `["kind", "branchId"]` — the value
the hand-written list did not have, and the reason the card exists.

**Two corrections to this card's own work, both mine.**

`tagged_union_name` matches any `|` after the assignment, which a plain record
with a nullable field also has and a string union has too. The variant pass was
calling it directly and reporting a dozen plain records as unreadable unions.
It now filters as `field_map` does — reject what `plain_object` accepts — and
additionally requires a brace, which no string union has.

The "unreadable under both keys" filter was two chained filters that always
produce an empty set. It should have been an intersection, and read as if it
worked because empty is what a healthy domain reports.

**One test bug worth recording.** The fixture type was named `Record`, and the
rendered constant's own annotation is
`Record<string, Record<string, readonly string[]>>`, so the assertion matched
the header rather than an entry.

**`field_map`'s skipped warning is no longer printed for this domain.** It
skips every union by design, so reporting it here now misleads: the tagged
unions are in the variant map and the string unions are in the `*_POSITIONS`,
`*_CODES` and `*_KINDS` constants. The eleven domains batch 2 has not reached
keep their warning.

Evidence:
- `a_tagged_union_yields_each_variants_keys_including_the_discriminant`,
  `a_nested_object_does_not_split_an_arm`,
  `documentation_between_arms_is_not_read_as_a_field`,
  `a_nullable_field_does_not_make_a_record_a_union`
  (`crates/longhorn-bindings/src/generation.rs`)
- `packages/longhorn/tests/history-tree/variant-fields.test.ts`, six tests,
  including `checkoutBranchRoot` accepted with its `branchId` and rejected
  without it
