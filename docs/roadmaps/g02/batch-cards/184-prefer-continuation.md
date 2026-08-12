# 184 Prefer A Continuation Without Walking Into It

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.016 batch 3
Governing refs: contract 011; contract 012
Depends on: Card 183
Blocks: Poodle's HistoryCentre v3 activation control
Auto-start next card: no

## Why

Card 183 lets an operator see the continuations at an entry and page into one.
This card lets them choose it.

The operator intent is narrow: make this fork the active future, without
applying any of it. The list they are reading swaps — the chosen fork becomes
the flat root list, and what was the root list becomes a fork at the same entry
— while the document does not move forward by a single delta.

Nothing expresses that today. `Checkout { branch_id, entry_id }` looks like it
should: checking out the fork's branch at the fork entry does switch branch
without applying anything. But `execute_navigation` re-points preferred children
only along the target lineage *down to the target node*, so checking out the
fork entry itself re-points nothing, `default_lineage` still walks the old
preferred child, and the futures do not swap. Checking out the fork's first
entry does swap them, at the cost of applying that entry — which is the thing
the operator asked not to do.

## Step 1 — The target

`src/navigation/types.rs`.

- [x] `ForkNavigationTarget::PreferContinuation { entry_id }`: make `entry_id`
      the preferred continuation of its parent.
- [x] Resolves to the parent as the target node and to the branch a consumer
      would land on by taking the child — the same derivation Card 183's
      continuation page reports, so the picker and the commit agree.
- [x] Committing sets `preferred_children[parent] = entry_id` in addition to
      whatever the route already re-points.
- [x] `entry_id` must be a retained entry. Its parent may be `None`: preferring
      a root continuation is the same operation at the root.

## Step 2 — Zero-step plans are legitimate here

`src/navigation/plan.rs:33` rejects a plan whose target node is the current node
as `AlreadyAtTarget`. That is right for every existing target and wrong for this
one — the whole point is to commit a change while standing still.

- [x] Exempt `PreferContinuation` from the `AlreadyAtTarget` check. Every other
      target keeps it.
- [x] A zero-step plan still bumps the revision and still returns a receipt.
      Consumers watch the revision, and a fork switch that did not move it would
      leave every page they hold looking current.
- [x] Reject preferring an entry that is already its parent's preferred child.
      That is genuinely nothing to do, and it is the one case where
      `AlreadyAtTarget` is the honest answer.

## Step 3 — The operator standing downstream

If the current node sits inside the subtree being replaced, re-pointing the
parent's preferred child would leave the current position off the default
lineage, and the flat list would no longer contain where the operator is.

- [x] Planning moves the operator to the parent first, as ordinary undo steps,
      exactly as a checkout to that entry would. The route is real work with
      real inverses; it is not a special case in execution.
- [x] The operator therefore never ends up forward of the fork point, and never
      ends up inside the fork. Both are the described behaviour: after
      activating, they stand at the fork entry and choose what to redo.

## Step 4 — Protocol and consumers

- [x] Carry the target through `ForkNavigationTargetProjection`, the receipt,
      the rejection projection and the generated TypeScript.
- [x] `ForkHistoryController.preferContinuation(entryId)`, refreshing snapshot,
      path and branches together the way Card 181 step 1 established.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] A navigation test standing at the fork entry prefers a continuation,
      applies zero steps, bumps the revision, and finds the default path page
      now walks the chosen fork.
- [x] A navigation test standing three entries downstream prefers a
      continuation and lands at the fork entry with three undo steps applied.
- [x] A navigation test asserts preferring the current preferred child is
      `AlreadyAtTarget`.
- [x] A navigation test asserts the previous future is still reachable — it is
      now a continuation at the same entry, with the same entry count.

## Evidence

- [x] The tests above, named in the batch log.

## Stop Conditions

- Stop if exempting one target from `AlreadyAtTarget` requires the check to
  move into each target's resolver. Spreading a guard across every variant to
  serve one of them is worse than the guard being wrong for one of them, and
  the alternative — a separate non-navigation mutation — is then the better
  trade.

## Continuation

Poodle implements HistoryCentre v3 against Cards 183 and 184. Return to the
milestone's planning checkpoint after this card.

## Outcome — 2026-08-12

Landed as planned. `PreferContinuation { entry_id }` resolves to the entry's
parent as the target node and to the branch a consumer would land on by taking
the child -- the same derivation the continuation page reports, so the picker
and the commit name the same branch. Execution re-points
`preferred_children[parent]` after the route has re-pointed everything down to
the target, which is the one step past the target that `Checkout` could never
take.

The `AlreadyAtTarget` exemption stayed a single `matches!` at the one existing
check, so step 2's stop condition never came into play. The genuinely-nothing-
to-do case moved into the resolver, where it belongs: preferring the entry that
is already preferred is rejected there.

Evidence:
- `preferring_a_continuation_applies_nothing_and_swaps_the_default_path`,
  `preferring_a_continuation_from_downstream_returns_to_the_fork_entry`,
  `preferring_the_continuation_already_preferred_is_rejected`,
  `preferring_a_continuation_that_does_not_exist_is_rejected`
  (`crates/longhorn-history-tree/tests/navigation_retention.rs`)
- `effigy qa` exit 0.
