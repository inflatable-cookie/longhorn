# g02.017 Fork Retention And Deletion

Status: ready
Owner: Tom
Governing refs: contract 011; contract 012; contract 017
Depends on: g02.016 (complete)

## Outcome

A fork history grows on every divergence and nothing ever removes one. Loophole
reports this from field use before it has shipped the feature widely: a session
of ordinary editing produces forks faster than an operator produces intent, and
none of them go away.

This milestone gives the operator a way to delete a fork they know they do not
need, and gives the host a way to keep the graph inside a budget without being
asked. Both are missing today, and the second is worse than missing — it is
present, unreachable, and would not work if it were reached.

## What Exists, And Why It Does Not Help

`ForkHistory::prune_to(revision, limits)` removes the oldest unprotected
**leaf**, one at a time, until the entry-count and encoded-weight budgets are
met. It returns a receipt naming pruned nodes, removed branches and removed
checkpoints. It is careful, tested code.

Three things stand between it and an operator clicking a remove button.

**There is no way to delete a named thing.** Pruning is budget-driven and
erodes from the tip. It cannot take a subtree. Nothing in the crate deletes a
branch on request.

**Retention has no surface above Rust.** No protocol command, no Tauri command,
no controller method. A host must call `prune_to` from its own Rust.
`ForkChangedKind::Retention` exists as an event kind, so the design expected a
host to prune and publish — but nothing carries the request inward.

**Automatic pruning cannot prune anything in Loophole.**
`retention.rs:255` protects the lineage of every branch that is current, or
pinned, **or has a name**:

```rust
branch.branch_id() == &self.current_branch_id
    || branch.metadata().name().is_some()
    || branch.metadata().pinned()
```

Loophole names every fork at divergence — `pulse-history/src/history.rs:385`
seeds each one `"Fork N"`, per the operator ruling of 2026-08-11. So every
branch is protected, every node is protected, `plan_pruning` finds no candidate
and `prune_to` returns `ProtectedBudget` the moment a budget is exceeded. It
never prunes.

Neither decision was wrong when it was taken. The name clause was written when
a name meant an operator had cared enough to type one. Auto-naming turned it
into "protect everything". `pinned` already means what protection should mean,
which makes the name clause redundant as well as harmful.

## Operator Decision — settled 2026-08-12

**Deleting a fork is irreversible, and the deletion is not itself undoable.**

The point of deleting a fork is that it stops taking up room. A delete that can
be undone has to keep everything it deleted, which is the state the operator
asked to leave. There is nothing to put back and nowhere cheaper to put it.

This makes the operation the only destructive one in either history domain, so
it is confirmed at the UI and it is reported precisely: the receipt names every
node, branch and checkpoint it removed, and the revision moves.

## Planning Gaps

- **Who sets the budgets, and when does automatic pruning run?** Longhorn owns
  no policy and no scheduler. The likely answer is that the host supplies
  limits and calls a command, but whether that call is on record, on a timer,
  or on an explicit operator action is a consumer decision that shapes the
  command's shape. Batch 2 stays paused until it is answered.
- **What happens when the protected set alone exceeds the budget?** Today it is
  `ProtectedBudget`, an error with no remedy the operator can act on. Once
  protection means `pinned` only, the remedy is "unpin something" — but the
  error does not say which, and the UI has nothing to show. Answer before
  batch 2.

## Execution Plan

- [ ] **Batch 1. Delete one fork** (Card 185). An explicit, irreversible
      subtree removal taking the same handle `CheckoutContinuation` takes, with
      a protocol command, a Tauri command and a controller method. Rejects
      deleting the line the operator is on or inside.
- [ ] **Batch 2. Retention that can act** (Card 186, paused on both gaps
      above). Drop the name clause from protection, then give `prune_to` a
      surface so a host can hold a budget without writing Rust.
- [ ] **Batch 3. Bulk selection, if the field asks for it.** Deleting forks one
      at a time is fine for a handful and wrong for two hundred. A
      delete-many-by-predicate belongs here, not in batch 1, and only once
      batch 1 has shown what an operator actually reaches for.

## Goals

- [ ] An operator can delete a fork they know they do not need, and the space
      it occupied is gone.
- [ ] A host can hold a fork graph inside a budget without calling into Rust.
- [ ] Automatic pruning prunes. Today it cannot, in the one consumer that has
      the problem.
- [ ] Nothing protects a branch merely because it has a name.

## Acceptance Criteria

- [ ] Deleting a fork removes its subtree, the branches whose heads are inside
      it, and the checkpoints inside it, and reports all three.
- [ ] Deleting the continuation the operator is standing on or inside is
      rejected, not silently redirected.
- [ ] A named, unpinned, non-current branch is prunable.
- [ ] A pinned branch is not prunable, whatever its budget pressure.
- [ ] `effigy qa` passes, including `check:bindings`.

## Explicit Non-goals

- No undo of a delete, and no tombstone. See the operator decision.
- No clock. Deleting "everything older than a week" needs a time, and the only
  time in this domain is the host-supplied `recorded_at` from Card 182. A host
  that stamps can compute that set itself and delete each one; Longhorn does
  not gain a predicate that reads a clock it does not own.
- No policy. Longhorn does not decide budgets, does not schedule pruning, and
  does not prune on record.

## Next Task

Card 185. Batch 2 waits for both planning gaps; batch 3 waits for batch 1 to
ship and for the field to say whether one-at-a-time is enough.

## Planning Checkpoint

After Card 185, before batch 2. The two gaps are consumer-policy questions and
the answer to the first one shapes the command.
