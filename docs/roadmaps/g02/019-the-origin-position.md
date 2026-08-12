# g02.019 The Origin Position

Status: in progress — batch 1 complete 2026-08-12
Owner: Tom
Governing refs: contract 011; contract 012; contract 017
Depends on: g02.016 (complete)

## Outcome

Every state an operator reaches by editing has a row in the history centre,
except the one they started from. This milestone gives the origin a name the
authority can navigate to and a fact a renderer can draw.

Reported from Loophole's field use: "each thing you do adds a history step, but
there's no step in the history centre to browse to from the state you started
with."

## What Is Actually Missing

Verified 2026-08-12. Three separate things, and they are not the same in the
two domains.

| | Fork | Linear |
| --- | --- | --- |
| Origin reachable directly | yes, `CheckoutBranchRoot` | **no** |
| Origin representable | yes, `currentEntryId: null` | yes |
| Origin has a row | **no** | **no** |

**The fork domain can reach it.** Card 181 step 3 added
`CheckoutBranchRoot { branch_id }` for exactly this position — "where a nascent
branch sits until something is recorded on it" — and it is forwarded on the
controller and the session. What it does not have is a row: `default_lineage()`
is `lineage(current)` plus the forward walk, and `lineage(None)` is empty, so
the origin contributes no entry to any page.

**The linear domain cannot reach it directly at all.**
`HistoryNavigationTarget` is `Undo | Redo | Checkout { entry_id }`, and the
`Checkout` field is documented "entry identity, never a presentation index".
`planning.rs:43` resolves undo as `source_depth.checked_sub(1)`, so depth zero
is the origin and it is reachable **one undo at a time**. Returning to the
state a two-hundred-entry document was opened in is two hundred undos.

The fork domain got its root target in Card 181. The linear domain never did,
and it is the domain most consumers use.

## The Difference That Nearly Shipped Wrong

The two domains disagree about whether the origin survives retention, and a row
that ignores that would lie to the operator.

**Fork: the origin is guaranteed.** `protected_lineage` covers the current
branch's whole lineage, root to head, so no entry on the current path is ever
pruned. The default path always runs back to the true origin.

**Linear: the origin can be gone.** Linear retention prunes the oldest entries
and records what it took in `HistoryBaselineProjection` —
`prunedEntryCount`, `prunedEncodedWeight`, `lastPrunedEntryId`,
`lastPrunedSequence`. After pruning, undoing to depth zero reaches the state
before the oldest *retained* entry, which is not the state the document was
opened in. Everything earlier is unrecoverable.

So the linear row is only truthful when `prunedEntryCount` is zero. Otherwise
the position below the oldest entry is a baseline, and calling it the origin
would be an invented claim about data the authority no longer holds.

## Operator Decision Needed

**None on the label.** The row's text is not the authority's business: a
document loaded from disk sits at its loaded state, not an empty one, and only
the host knows which. The label is a Poodle prop the host supplies, the same
way it supplies every other piece of presentation. Longhorn carries no string
for it.

That is a decision this milestone takes rather than defers, because the
alternative — a label on the protocol — would put presentation in the
authority for one row.

## Execution Plan

- [x] **Batch 1. The authority can name the origin** (Card 191, complete
      2026-08-12). A linear
      `CheckoutRoot` target, and both domains stating on the page whether the
      position below the oldest entry is the origin or a baseline.
- [ ] **Batch 2. Consumers draw it.** Poodle renders a row that is not an
      entry, can be current, and navigates to the origin; Loophole supplies its
      label and grants nothing new. Tracked here, owned there.

## Goals

- [ ] An operator can return to the state they started in with one action, in
      both domains.
- [ ] The history centre shows that state as a row, marked current when the
      operator is there.
- [ ] A pruned linear history does not claim an origin it no longer holds.

## Acceptance Criteria

- [x] `HistoryNavigationTarget::CheckoutRoot` exists, is refused when already
      at depth zero, and unwinds every applied entry in one plan.
- [x] Both page snapshots say what sits below their oldest entry.
- [x] A linear page whose baseline is non-empty says baseline, not origin.
- [x] A fork page always says origin on the default path, whatever retention
      has done elsewhere in the graph.
- [x] `effigy qa` passes, including `check:bindings`.

## Explicit Non-goals

- No label, no icon, no ordering advice. The row is the renderer's; this
  milestone supplies the position and the truth about it.
- No new position in the graph. The origin already exists in both domains and
  is already reachable; nothing here adds a node.

## Next Task

Batch 2, in Poodle and Loophole. The authority now names the origin and says
whether it is one; nothing renders it yet.

## Planning Checkpoint

After batch 1, before Poodle draws anything. The baseline distinction is the
part most likely to be rendered wrong, and one worked consumer will show
whether the page says enough.
