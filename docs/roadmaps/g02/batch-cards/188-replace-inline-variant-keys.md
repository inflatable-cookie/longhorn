# 188 Replace The Inline Variant Key Lists

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.018 batch 2
Governing refs: contract 010; contract 011; contract 012
Depends on: Card 187 (complete)
Blocks: batch 3
Auto-start next card: no

## Why

Three domains already validate tagged unions per variant, and all three write
the key lists by hand:

| Domain | Inline lists |
| --- | --- |
| `native-content` | 32 |
| `operation` | 25 |
| `notifications` | 12 |

They are literal arrays passed straight to `exactKeys` inside a `switch`, not
named constants. That is the same second copy of the enum that Card 187
removed from `history-tree`, in a shape that does not look like a map. Card
187's argument applies unchanged: a hand-written key list drifts, and
`checkoutBranchRoot` proved it drifts silently.

Replacing them is not a behaviour change unless a list is already wrong. If one
is, this card finds it, which is the reason to do these three before the eight
that have no checks at all.

## Scope

`native-content`, `operation`, `notifications`. Their generators, and their
validation modules. The eight lenient domains are batch 3 — they gain
strictness rather than exchanging it, and that wants separate evidence.

## Step 1 — Emit the map in the three generators

`crates/longhorn-bindings/src/{native_content,operation,notifications}.rs`.

- [x] Each renders a variant map beside its flat one, using
      `variant_field_map` as `history_tree.rs` does. No new generator code:
      Card 187 built the function to be called from twelve places.
- [x] Check each domain's discriminant key before assuming `kind`.
      `history-tree` needed two passes because its results are tagged `status`,
      and a domain whose unions disagree needs the same treatment.
- [x] A union readable under none of the keys tried is reported by name. Do not
      let it fall through to an absent entry, which reads at the boundary as
      "no keys allowed".

## Step 2 — Read the map at every call site

- [x] Each `exactKeys(object, path, [ ... ])` inside a discriminated `switch`
      becomes a lookup by type and discriminant.
- [x] Keep the existing `member`/`oneOf` check on the discriminant above the
      lookup. A missing map entry then means the generator failed, not that a
      consumer sent something unexpected — the same ordering Card 187 fixed.
- [x] Where a `switch` has a `default` arm that accepts anything, it becomes
      unreachable once the discriminant is checked above. Delete it rather than
      leaving it as a fallback.

## Step 3 — Report what the generator disagreed with

The interesting output of this card is not that 69 sites compile. It is
whether any of the 69 disagreed with the enum.

- [~] Every difference between a hand-written list and the generated one is
      named in the batch log, with which was right. **Partly done — see the
      outcome.** A sound per-variant comparison exists for `operation` only.
- [x] A list that was too narrow was rejecting valid payloads at a boundary.
      That is a shipped defect and it gets a test, as `checkoutBranchRoot` did.
- [x] A list that was too wide was accepting unknown keys. Lower severity —
      the payload still had to satisfy every field check below — but it is
      still a boundary that was not doing its job.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] No literal key array is passed to `exactKeys` in any of the three
      domains' validation modules.
- [x] Each domain has a test that sends one variant with exactly its declared
      keys and is accepted, and the same variant with one extra key and is
      rejected.
- [x] Any disagreement found in step 3 has a test naming the payload that was
      wrongly handled.
- [x] The generator reports no unreadable union in these three domains.

## Evidence

- [x] The tests above, named in the batch log.
- [x] The disagreement list from step 3, including "none" if that is the
      answer. A card that touched 69 hand-written lists and found no drift is
      worth recording as evidence that the drift risk is lower than argued.

## Stop Conditions

- Stop if a domain's unions do not share one discriminant across the domain and
  the number of passes needed exceeds two. `history-tree` needed `kind` and
  `status`; a domain needing five is telling us the enums disagree about their
  own shape, and that is a modelling decision rather than this card's.
- Stop if any hand-written list is wider than the generated one *and* a
  consumer depends on the extra key. That is a protocol question — either the
  key belongs on the enum or the consumer is sending something the authority
  never declared — and it is the operator's.

## Continuation

Batch 3 adds per-variant checks to the eight domains that have none. That one
can reject payloads those boundaries used to accept, so it wants a planning
checkpoint first.

## Outcome — 2026-08-12

All 69 lists replaced across the three domains, zero literal key arrays left in
any of them. `effigy qa` exit 0; 194 package tests plus five new ones.

**Two generator changes the domains forced, both improvements on Card 187.**

The discriminant is now *detected* rather than supplied. Card 187 had the
caller pass the tag, which was tolerable for one domain and wrong for this one:
`native-content` uses `kind`, `status` and `state`. `ts-rs` quotes only the tag
key and its value, so the tag is the one quoted-literal key every arm shares,
and that is readable from the declaration. `history-tree`'s two maps collapsed
back to one as a result.

The discriminant's *name* is emitted beside the map, so a call site passes the
object rather than a chosen property. One less per-site chance to name the
wrong one, in a domain that offered three wrong ones.

**Two sites were already approximating.** `operation`'s `executorDispatch`
else-branch used one list for `notRequired` and `requested`, correct only
because they happen to be identical, and `notifications` used a ternary for its
clear target. Both now resolve per variant.

**Step 3 is partly done, and the shortfall is mine.**

The card asked for every difference between a hand-written list and the
generated one. Attributing a literal to its variant means following
`switch`/`case`, `if`/`else if`/`else` and one ternary by parsing, and three
attempts at that each produced false positives -- an `else` branch's keys
reported against the `if` branch's variant, and variant names matched across
unrelated unions. Reporting parser noise as findings would be worse than
reporting less.

What is sound:

- `operation`, read line by line with variant names taken from the source:
  **25 of 25 match.** Two apparent mismatches were my own typos of the variant
  names, not disagreements.
- All three domains, without attribution: **all 68 comparable literal lists are
  key sets the authority declares somewhere in that domain.** A list appearing
  nowhere would be certain drift; none does. This cannot catch two variants'
  lists being swapped, which the passing suites make unlikely but do not
  disprove.

So: no evidence of drift, at a weaker standard than the card set. Batch 3
should not inherit the assumption that these 69 were all correct -- it should
inherit that nobody checked 44 of them per variant.

**The new tests found a defect in their own fixtures, not in the code.** The
first draft invented an `authority` of `{ kind, windowId }`; every domain
declares `{ authorityId, authorityEpoch }`. The boundary rejected it, which is
the boundary working.
