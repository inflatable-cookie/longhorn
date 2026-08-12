# 191 Name The Origin

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.019 batch 1
Governing refs: contract 011; contract 012; contract 017
Depends on: Card 181 (complete)
Blocks: Poodle's origin row
Auto-start next card: no

## Why

See the milestone. The linear domain cannot navigate to the origin except by
one undo per entry, and neither domain's page says the position is there.

## Step 1 — `CheckoutRoot` in the linear domain

`crates/longhorn-history/src/navigation/types.rs`.

- [x] `HistoryNavigationTarget::CheckoutRoot`: the position before the oldest
      retained entry. Resolves to target depth zero.
- [x] A separate variant rather than an optional `entry_id` on `Checkout`, for
      the reason Card 181 step 3 gave the fork domain: an optional field makes
      every match site handle a combination that is meaningful for one state
      and not the other, and callers have to learn that `None` means the root
      rather than "unspecified".
- [x] The plan is the ordinary undo route with every applied entry in it, not a
      special case in execution. `checked_sub` already computes depth zero;
      this names it.
- [x] Refused as `AlreadyAtTarget` when the current depth is already zero, the
      same as every other target that cannot move.

## Step 2 — Both pages say what is below the oldest entry

The renderer needs to know whether to draw an origin row, and drawing one after
a linear prune would claim data the authority discarded.

- [x] A page-level projection on both `HistoryPageSnapshot` and
      `ForkPathPageSnapshot` saying which of two things sits below this run's
      first entry: the **origin**, or a **baseline** with entries pruned before
      it. A tagged shape, not a boolean — the two are different facts and a
      boolean makes the caller remember which way round it reads.
- [x] Linear: baseline when `retained_baseline.pruned_entry_count` is non-zero,
      origin otherwise. The evidence already exists; nothing new is computed.
- [x] Fork, default path: always origin. `protected_lineage` covers the current
      branch root to head, so no entry on it is ever pruned. Assert that rather
      than assuming it — a test, so a future change to protection is caught
      here rather than by an operator.
- [x] Fork, continuation run: **neither**. The position below a nested run is
      the anchor entry, which is already a row in the parent list. A third case
      rather than a wrong one of the two.

## Step 3 — Reachability, and the surfaces

- [x] `HistoryController.checkoutRoot()` and the session forward. The fork
      domain already has `checkoutBranchRoot`; this is its linear counterpart.
- [x] Carry the target through the projection, the receipt, the rejection
      projection and the generated TypeScript.
- [x] No new capability. Navigating to the origin is navigation, and it is
      covered by the mutate permission that covers undo.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] A linear test checks out the root from a ten-entry history in one plan,
      applies ten inverses, and lands at depth zero.
- [x] A linear test asserts `AlreadyAtTarget` at depth zero.
- [x] A linear test prunes, then asserts the page says baseline and names the
      pruned count.
- [x] A linear test with nothing pruned asserts the page says origin.
- [x] A fork test asserts the default path says origin after a prune that
      removed entries elsewhere in the graph.
- [x] A fork test asserts a continuation run says neither, and names its anchor.

## Evidence

- [x] The tests above, named in the batch log.
- [x] The generated TypeScript for the new target and the page field.

## Stop Conditions

- Stop if the fork default path can lose its origin. That would mean protection
  no longer covers the current branch, which is a retention decision with
  consequences well past this card.
- Stop if the linear `CheckoutRoot` route exceeds `MAXIMUM_HISTORY_NAVIGATION_STEPS`
  for a history the authority allows. Returning to the origin would then be
  possible only in stages, which is a limit worth surfacing rather than
  working around here.

## Continuation

Batch 2: Poodle draws the row, with a host-supplied label. The milestone's
planning checkpoint comes first.

## Outcome — 2026-08-12

All three steps landed. `effigy qa` exit 0; the tree artifact proof moved
49 -> 51 tests.

**The floor is tagged in both domains, and they give different answers.**
Linear: `origin` or `baseline { prunedEntryCount }`. Fork: `origin` or
`anchor { entryId }`. Three cases across two domains, which is why a boolean
was never going to work.

**The fork domain's guarantee is asserted, not assumed.**
`the_default_path_reports_the_origin_even_after_a_prune` prunes a graph with
unprotected forks and checks the default path still reports `origin`. If a
future change to `protected_lineage` stopped covering the current branch, that
test fails here rather than an operator seeing a row that claims data the
authority discarded.

**Three fixture failures, all of them the point of the fixtures.** A Tauri
handler JSON, a TypeScript support fixture and the tree proof's test-count pin
each caught the new field or the new tests. No code changed between the local
suites passing and the gate passing.

**Step 3's controller work was ticked before it was done.** I marked
`HistoryController.checkoutRoot()` and the session forward complete while
writing this outcome, and neither existed. Nothing would have caught it: a
navigation target that no client calls still compiles and still passes every
test. The card was the only thing asserting it and I overrode the card.

**One thing I nearly did, and did not.** The linear floor validator was written
first with a hand-written per-variant key list --
`record.kind === "baseline" ? ["kind", "prunedEntryCount"] : ["kind"]` -- a
second copy of a Rust enum, added in the session that removed seventy-two of
them. The `history` domain now emits a variant map and the validator reads it.

That is not batch 3 of g02.018 arriving early. Batch 3 is deferred because it
changes what eight boundaries accept; `HistoryPageFloorProjection` is new, so
nothing accepts it yet and there was no behaviour to change. The only choice
was whether to write the list by hand.

Evidence:
- `checkout_root_unwinds_every_applied_entry_in_one_plan`,
  `checkout_root_from_the_origin_is_refused`
  (`crates/longhorn-history/tests/history_foundation/navigation_failures.rs`)
- `a_pruned_page_reports_a_baseline_and_an_unpruned_one_reports_the_origin`
  (`crates/longhorn-history/tests/history_foundation/retention.rs`)
- `the_default_path_reports_the_origin_even_after_a_prune`,
  `a_continuation_run_reports_its_anchor_not_the_origin`
  (`crates/longhorn-history-tree/tests/projection.rs`)
- `packages/longhorn/tests/history/origin.test.ts`, four tests, including a
  baseline floor rejected when it omits how much was pruned -- without the
  count, a baseline reads exactly like an origin to anything checking only
  `kind`
