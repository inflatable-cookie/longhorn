# 186 Retention That Can Prune

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.017 batch 2
Governing refs: contract 011; contract 012; contract 017
Depends on: Card 185
Blocks: any consumer holding a fork graph inside a budget

## Why

`prune_to` cannot prune anything in Loophole, and the budget it enforces means
the wrong thing. Two independent defects that happen to cancel into one symptom:
the graph grows and nothing stops it.

## Operator Decisions — settled 2026-08-12

**Protected entries fall outside the budget.** Once protected, an entry is a
core record of the project, not a transient budgeted thing. The budget governs
how much *transient* history is kept, not how large the graph is.

This deletes a whole failure mode. `ProtectedBudget` — "the protected lineage
alone exceeds the requested budget" — is not a condition that can arise. If
everything is protected there is nothing to prune and nothing is over budget.
It was an error with no remedy an operator could act on, and now there is
nothing to remedy.

It also means a graph can grow without bound through pinning. That is correct
and intended. The operator pinned those.

**Pruning has three triggers, and the app chooses which it uses.** On record,
on a timer, and on an explicit operator action. Per-app, opt in or out of each.

## Step 1 — The budget counts unprotected entries only

`src/retention.rs`.

- [x] `plan_pruning` compares the **unprotected** count and weight against the
      limits, not the totals. Today it loops while the whole graph exceeds the
      budget and then looks for an unprotected leaf to remove, which is how a
      fully protected graph reaches `ProtectedBudget`.
- [x] `prune_to`'s early return does the same: unchanged when the unprotected
      share already fits.
- [x] Delete `ForkRetentionError::ProtectedBudget`. Not deprecated — removed.
      Pre-1.0, and leaving an unreachable variant invites a consumer to handle
      a case that cannot happen.
- [x] `ForkRetentionLimits`' documentation says what it now bounds. The type
      keeps its name and its fields; the meaning changed from "how large the
      graph may be" to "how much unprotected history is kept", and a reader who
      misses that will size it wrong by the size of their pinned set.
- [x] `ForkPruningReceipt` reports the retained totals it always did. Add the
      unprotected share beside them, so a host can see the number the budget
      actually governs.

## Step 2 — A name is not protection

`retention.rs:255` protects the lineage of every branch that is current, or
pinned, **or has a name**. Loophole names every fork at divergence, so every
branch is protected.

- [x] Drop the name clause. Protection is the current branch plus pinned
      branches.
- [x] This is a behaviour change to shipped code, and the right one: `pinned`
      exists to mean protect, and a name written by an auto-namer carries no
      operator intent at all. A consumer that relied on names protecting
      branches was relying on an accident.
- [x] Step 1 alone does not fix Loophole. Under step 1 a fully protected graph
      simply never prunes instead of erroring, which is quieter and equally
      useless. Both steps are needed and neither is sufficient.

## Step 3 — A surface

- [x] `ForkPruneCommand` carrying the limits, and a receipt projection.
      Protocol version, authority epoch, expected revision, like every other
      command.
- [x] A Tauri command with a named re-export, under the destructive capability
      Card 185 introduces rather than the mutate one. Pruning removes entries.
- [x] `ForkHistoryController.prune(limits)`, forwarded on `ForkHistorySession`.
- [x] Publish `ForkChangedKind::Retention`. Card 185 is the first thing to
      construct it; this is the second.

## Step 4 — Longhorn owns no scheduler

The three triggers are the host's, and Longhorn holds no configuration for
them. See the milestone's note on why, and confirm it still reads true after
step 3 exists.

- [x] Say so once where a host integrator reads it: the crate docs or the
      getting-started guide. The three triggers, and that all three are the
      host calling one command.
- [x] `ForkSummaryProjection` already carries `retained_entry_count` and
      `retained_encoded_weight`, so a host can already see budget pressure
      after any record without a new signal. Confirm that is still true and
      name it in the same place, because it is what makes the on-record
      trigger a host concern rather than a hook.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] A test builds a graph where every branch is named and unpinned, sets a
      budget below the entry count, and asserts pruning removes entries. This
      is the Loophole shape and it is the reason the card exists.
- [x] A test pins a branch, sets a budget below the pinned lineage's own size,
      and asserts `Unchanged` — not an error, and not a pruned pin.
- [x] A test asserts the budget is measured against the unprotected share: a
      graph whose protected set is large and whose unprotected set fits is
      `Unchanged`.
- [x] A test asserts `ProtectedBudget` no longer exists, by the enum not
      compiling with it — no test needed beyond the removal itself.
- [x] A conformance test drives the prune command end to end and asserts a
      `Retention` changed event is published.

## Evidence

- [x] The tests above, named in the batch log.

## Stop Conditions

- Stop if measuring the unprotected share requires walking every branch lineage
  on each loop iteration. `protected_lineage` is computed once today; if the
  new comparison forces it per-candidate the cost changes class, and that is a
  performance decision rather than this card's.
- Stop if dropping the name clause makes any shipped consumer's pinned set
  empty and their whole graph prunable. That is a data-migration question about
  their stored pins, not a code question, and it is the operator's.

## Continuation

Batch 3 is bulk selection, and only if the field asks for it. Return to the
milestone's planning checkpoint after this card.

## Outcome — 2026-08-12

All four steps landed. `effigy qa` exit 0; the tree artifact proof moved
47 -> 49 tests.

**`ProtectedBudget`'s removal is provable, not just unused.** The fallback that
raised it is unreachable for a structural reason, and the reason is recorded
where the `expect` now sits: protection is by lineage, so the protected set is
ancestor-closed, so the unprotected set is descendant-closed, so any
unprotected entry has an unprotected leaf below it. A candidate always exists
while the loop runs.

**The receipt reports the share, recomputed after the removal.** Pruning can
take a branch with it, which changes what is protected, so carrying the
pre-removal figure through would have reported a number that was true before
the operation and not after.

**Three tests replaced one.** `named_pinned_and_current_lineages_reject_impossible_budgets_without_mutation`
pinned the removed error across three fixtures. It became
`a_named_unpinned_branch_is_prunable` — the Loophole shape, and the reason the
card exists — plus `a_pinned_branch_survives_any_budget` and
`the_budget_is_measured_against_the_unprotected_share`.

**The changed-event criterion was ticked before it was met.** The handler test
returned `Unchanged`, which publishes nothing, so the event side of both
destructive commands was untested. Both were also building the same event
inline. It is now `fork_retention_changed_event`, a pure function beside the
existing `fork_history_changed_event`, asserted directly -- which removed the
duplication and made the event testable without driving an emit.

**Two other places had sized budgets against the whole graph.**
`deterministic_pruning_removes_only_anonymous_unpinned_future` used `(3, 24)`,
which now reads as "keep three unprotected entries" and was already satisfied.
And the packed artifact proof asserted `protectedPruneRejectedWithoutMutation`
— it pinned the removed error too. Both now express the new meaning, and the
proof's reported flag is renamed `protectedBudgetLeftGraphUnchanged` rather
than left saying "rejected" while reporting that nothing happened.

Evidence, in `crates/longhorn-history-tree/tests/navigation_retention.rs`:
- `a_named_unpinned_branch_is_prunable`
- `a_pinned_branch_survives_any_budget`
- `the_budget_is_measured_against_the_unprotected_share`
- `deterministic_pruning_removes_only_anonymous_unpinned_future`, resized
