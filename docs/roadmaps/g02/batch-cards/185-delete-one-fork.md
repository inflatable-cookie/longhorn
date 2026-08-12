# 185 Delete One Fork

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.017 batch 1
Governing refs: contract 011; contract 012; contract 017
Depends on: Card 184 (complete)
Blocks: Poodle's remove control beside checkout in HistoryCentre v3
Auto-start next card: no

## Why

Nothing in either history domain deletes a named thing. `prune_to` erodes
unprotected leaves against a budget; it cannot take a fork the operator points
at, and in Loophole it cannot take anything at all — see the milestone.

The operator has knowledge the budget does not. They know the fork they made by
accident, and the one they explored and rejected. A remove control beside
checkout lets them act on it. That control needs something to call.

## Operator Decision — settled 2026-08-12

Deleting a fork is irreversible, and the deletion is not itself undoable. A
delete that can be undone has to keep what it deleted, which is the state the
operator asked to leave.

So this is the only destructive operation in either history domain. It is
confirmed at the UI, and it reports exactly what it removed.

## Scope

`longhorn-history-tree`, plus the Tauri crate and the TypeScript client. The
linear domain has no forks and is untouched.

## Step 1 — The operation

`src/retention.rs`, beside `prune_to`, because it shares the receipt and the
same structural-removal care.

- [x] `delete_continuation(expected_revision, entry_id)` removes `entry_id` and
      its whole subtree.
- [x] Same handle `CheckoutContinuation` takes: the fork's first entry. The
      picker already holds it, so the remove control needs nothing new, and the
      two operations name the same thing the same way.
- [x] Reuse `ForkPruningReceipt`. It already reports pruned nodes, removed
      branches and removed checkpoints, which is exactly the three things a
      subtree removal takes with it. Do not invent a second receipt shape.
- [x] Remove every branch whose head is inside the subtree, and every
      checkpoint whose anchor is inside it. Both already happen in the pruning
      loop; the difference is which nodes are chosen, not what removal means.
- [x] Maintain the preferred-child invariant. If the deleted entry was its
      parent's preference, the parent must name another child or have none
      left. `prune_to` already does this at `retention.rs:188-204`; the same
      code path must cover this one.

## Step 2 — What it refuses

Each of these is a distinct rejection, not one error with a message.

- [x] The current node is inside the subtree. Deleting the ground the operator
      stands on is not a delete, it is a delete plus an unrequested navigation.
      Reject and let the consumer move first.
- [x] The entry is on the current path. Same reason, stated structurally: you
      cannot delete the line you are on.
- [x] The branch is pinned. Pinning means protect, and it must mean it against
      an explicit request as well as against a budget. An operator who wants it
      gone unpins it first, which is a second deliberate act for a destructive
      one.
- [x] `UnknownEntry` for an entry that is not retained.
- [x] ~~Deleting the last continuation of the root, leaving an empty graph, is
      **allowed**.~~ Wrong, and corrected during execution: standing at the
      root with one continuation, that continuation *is* the active line, so
      the refusal above already covers it and is the more important rule. See
      the outcome.

## Step 3 — Surface

- [x] `ForkDeleteContinuationCommand` and a receipt projection, following the
      existing commands exactly — protocol version, authority epoch, expected
      revision.
- [x] A Tauri command with a named re-export, per Card 181 step 2. Its
      capability must be separate from the read and mutate ones already there:
      a destructive operation is not covered by permission to navigate.
- [x] `ForkHistoryController.deleteContinuation(entryId)`, refreshing snapshot,
      path and branches together the way Card 181 step 1 established, and
      forwarded on `ForkHistorySession` — the session forwards by hand and has
      dropped four methods already.
- [x] Publish `ForkChangedKind::Retention`, which exists and is unused. This is
      the event it was added for.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] A test deletes a fork of three entries and asserts the receipt names all
      three nodes, its branch, and any checkpoint inside it.
- [x] A test deletes a fork that itself has a fork, and asserts the whole
      subtree goes, including the inner branch.
- [x] A test asserts the parent's `continuation_count` drops by exactly one,
      and that the parent keeps a valid preferred child.
- [x] A test asserts each of step 2's four refusals, by distinct error.
- [x] A test asserts the entry count and encoded weight both fall by the
      subtree's share — the space is actually gone, which is the whole point.
- [x] A test deletes the only continuation of the root and is **refused**, on
      the active-line rule. Deletion can never empty a graph.

## Evidence

- [x] The tests above, named in the batch log.

## Stop Conditions

- Stop if deleting a subtree cannot maintain the preferred-child invariant
  without a second pass over the whole graph. The invariant became a guard on
  2026-08-12 and a delete that violates it makes every forward walk truncate;
  a rebuild-everything fix is a performance decision, not this card's.
- Stop if the destructive capability cannot be separated from the existing
  mutate capability without breaking a shipped consumer's manifest. That is a
  consumer-facing permission change and the operator's call, not this card's.

## Continuation

Card 186 gives `prune_to` a surface and fixes the protection rule, once the
milestone's two planning gaps are answered. It does not auto-start.

## Outcome — 2026-08-12

All three steps landed. `effigy qa` exit 0; the tree artifact proof moved
40 -> 47 tests.

**One acceptance criterion was wrong and was changed, not worked around.**

The card asked for deleting the last continuation of the root to empty the
graph. It cannot. Standing at the root with one continuation, that continuation
*is* the active line -- the operator's entire future -- so the active-line
refusal already covers it, and that refusal is the more important rule. It is
the exact destructive accident worth preventing.

So deletion can never empty a graph: the active line always survives and there
is always an active line. An empty history is still a real state; it is where a
history starts, not somewhere this operation can take one. The test asserts the
refusal and records why.

**The removal pass is shared, not duplicated.** `prune_to`'s per-node loop moved
into `remove_in_leaf_order`. Budget pruning and explicit deletion choose
different nodes, but removing one means the same thing either way, and a second
copy would have drifted. Deleting feeds it the subtree in reversed pre-order, so
every node is childless when reached -- the invariant the loop already assumed.

**The current branch shrinks rather than being removed.** If its head is inside
the subtree and the operator is not, the branch is reset to the operator's
position. Without this the shared loop would remove the branch the operator is
standing on. `prune_to` never hit it because protection covers the current
branch; deletion can, so `ForkBranch::clear_head` exists now.

**Deletion has its own capability.** `allow-longhorn-history-tree-delete`, its
own permission file, and an example capability granted to one window.
`deletion_is_its_own_capability` asserts it grants nothing else and that
neither read nor mutate carries it.

Evidence, in `crates/longhorn-history-tree/tests/navigation_retention.rs`:
- `deleting_a_fork_removes_its_subtree_its_branch_and_its_checkpoints`
- `deleting_a_fork_takes_the_forks_inside_it`
- `deleting_refuses_the_ground_the_operator_stands_on`
- `deleting_refuses_the_active_line`
- `deleting_refuses_a_pinned_branch`
- `deleting_refuses_an_entry_that_does_not_exist`
- `deleting_the_only_continuation_of_the_root_is_refused`
