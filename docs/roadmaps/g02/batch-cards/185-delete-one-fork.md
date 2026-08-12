# 185 Delete One Fork

Status: ready
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

- [ ] `delete_continuation(expected_revision, entry_id)` removes `entry_id` and
      its whole subtree.
- [ ] Same handle `CheckoutContinuation` takes: the fork's first entry. The
      picker already holds it, so the remove control needs nothing new, and the
      two operations name the same thing the same way.
- [ ] Reuse `ForkPruningReceipt`. It already reports pruned nodes, removed
      branches and removed checkpoints, which is exactly the three things a
      subtree removal takes with it. Do not invent a second receipt shape.
- [ ] Remove every branch whose head is inside the subtree, and every
      checkpoint whose anchor is inside it. Both already happen in the pruning
      loop; the difference is which nodes are chosen, not what removal means.
- [ ] Maintain the preferred-child invariant. If the deleted entry was its
      parent's preference, the parent must name another child or have none
      left. `prune_to` already does this at `retention.rs:188-204`; the same
      code path must cover this one.

## Step 2 — What it refuses

Each of these is a distinct rejection, not one error with a message.

- [ ] The current node is inside the subtree. Deleting the ground the operator
      stands on is not a delete, it is a delete plus an unrequested navigation.
      Reject and let the consumer move first.
- [ ] The entry is on the current path. Same reason, stated structurally: you
      cannot delete the line you are on.
- [ ] The branch is pinned. Pinning means protect, and it must mean it against
      an explicit request as well as against a budget. An operator who wants it
      gone unpins it first, which is a second deliberate act for a destructive
      one.
- [ ] `UnknownEntry` for an entry that is not retained.
- [ ] Deleting the last continuation of the root, leaving an empty graph, is
      **allowed**. An empty history is a real state — it is where every history
      starts.

## Step 3 — Surface

- [ ] `ForkDeleteContinuationCommand` and a receipt projection, following the
      existing commands exactly — protocol version, authority epoch, expected
      revision.
- [ ] A Tauri command with a named re-export, per Card 181 step 2. Its
      capability must be separate from the read and mutate ones already there:
      a destructive operation is not covered by permission to navigate.
- [ ] `ForkHistoryController.deleteContinuation(entryId)`, refreshing snapshot,
      path and branches together the way Card 181 step 1 established, and
      forwarded on `ForkHistorySession` — the session forwards by hand and has
      dropped four methods already.
- [ ] Publish `ForkChangedKind::Retention`, which exists and is unused. This is
      the event it was added for.

## Acceptance

- [ ] `effigy qa` passes, including `check:bindings`.
- [ ] A test deletes a fork of three entries and asserts the receipt names all
      three nodes, its branch, and any checkpoint inside it.
- [ ] A test deletes a fork that itself has a fork, and asserts the whole
      subtree goes, including the inner branch.
- [ ] A test asserts the parent's `continuation_count` drops by exactly one,
      and that the parent keeps a valid preferred child.
- [ ] A test asserts each of step 2's four refusals, by distinct error.
- [ ] A test asserts the entry count and encoded weight both fall by the
      subtree's share — the space is actually gone, which is the whole point.
- [ ] A test deletes the only continuation of the root and gets an empty
      graph, not an error.

## Evidence

- [ ] The tests above, named in the batch log.

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
