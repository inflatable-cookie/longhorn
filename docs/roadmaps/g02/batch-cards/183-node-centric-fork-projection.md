# 183 Node-Centric Fork Projection

Status: complete — landed 2026-08-12
Owner: Tom
Roadmap: g02.016 batch 3
Governing refs: contract 011; contract 012
Depends on: Card 182 (complete)
Blocks: Card 184; Poodle's HistoryCentre v3
Auto-start next card: no

## Why

The milestone originally scoped this batch as "a single paged `ForkTreePage` in
ancestry order with branch and lane annotations". That shape was inferred from
Poodle's HistoryCentre v2 stitcher, which builds exactly that. Reading the
stitcher — `poodle/packages/core/src/history-center.ts` — showed the shape is
the problem, not the missing projection.

v2 renders a fork run inline between two spine rows, at one extra indent level.
Its `childrenOf` map is a vector, so it already supports N runs attached at one
entry, and it emits them back to back. Two forks at one entry and a fork off a
fork land at the same position in the row array, separated only by `depth` and
`lane.parentBranchId` -- one indent step, and a field no indentation shows. Past
`HISTORY_TREE_DEPTH_CAP` the depth saturates and even that step is gone.

The row data is therefore not ambiguous; the visual encoding is. Which is worse
in a way, because the fix looks like a rendering job. It is not: the encoding
is weak because a tree can only draw one fork per node unambiguously, and
multiple forks at one node are ordinary here. That is a shape problem, and
moving the stitch into the authority would not touch it.

The operator model is flat. There is one active list of deltas. Jumping back
and editing creates a fork; the entry that was jumped to now has more than one
continuation, and the old future is one of them. What the operator needs at
that entry is a count, a picker and a way to switch — not a second dimension in
the list.

So the projection is node-centric. A run is flat and each of its entries says
how many entries continue from it. Continuations at one entry are their own
bounded page, fetched when the operator opens that entry. A selected
continuation projects as another flat run, structurally identical to the first.
Recursion, not nesting: the same two projections describe the whole graph at
any depth, and no page ever has to encode lanes, depth or attachment.

## Scope

`longhorn-history-tree` only. The linear domain has no forks and is untouched.

## Step 1 — Divergence relative to the parent run

`src/projection/project.rs:92`. `ForkBranchProjection::divergence_entry_id` zips
the branch's lineage against the **current branch's** and reports the last
shared entry. For a branch that forked off another branch, that reports an
entry on the current path, collapsing two structurally different forks onto one
node. It is why v2's stitcher refuses to use the field and derives attachment
itself.

- [x] Compute divergence against the branch's nearest ancestor branch: among
      the other branches, the one sharing the longest lineage prefix. Report
      that branch's id alongside the entry, so a consumer can see what a branch
      forked off as well as where.
- [x] `None` for a branch that shares nothing, and for the branch that has no
      ancestor. Root-attached is a real state, not an error.
- [x] Branch counts are bounded by `MAXIMUM_FORK_BRANCHES`, so the pairwise
      comparison is acceptable. Do not add an index for it.

## Step 2 — `continuation_count` on every projected entry

- [x] `ForkEntryProjection` and `ForkEntryRecord` carry
      `continuation_count`: how many entries continue from this one.
- [x] Computed from the graph alone — `child_ids(entry).len()` — with no page
      context. A run always continues to exactly one of them, so a renderer's
      fork badge is `continuation_count - 1`, and a run's last entry always has
      zero children because the preferred chain only terminates at a childless
      node.
- [x] `ForkPathPage` carries `preceding_continuation_count` for the same
      reason one position above the run's first entry — the history root for a
      default or branch path, the anchor entry for a continuation run. It
      shipped as `root_continuation_count` and reported the history root's
      children on every page, which is a fact about a different position on a
      run page. Corrected on the Poodle thread's report; see the outcome.

## Step 3 — The continuations page

- [x] `project_continuations(anchor: Option<&HistoryEntryId>, request)` returns
      a bounded `ForkContinuationPage`: every child of the anchor, in stable
      graph order, including the one the caller is already showing inline. The
      projection is not told which that is, so the page and the count above can
      never disagree; the renderer filters the entry it already has.
- [x] Each `ForkContinuation` carries the child `entry_id` and label, its
      `recorded_at`, whether it is the `preferred` child, the `entry_count` of
      the run it starts, and the `branch_id` plus `branch_name` a consumer would
      land on by taking it.
- [x] The branch is derived by following preferred children from the child to a
      leaf and naming the branch whose head that is. Every leaf is a branch head
      — recording always sets the head of the branch it commits to — so the
      derivation is total.
- [x] An anchor that is not a retained entry is `UnknownEntry`, not an empty
      page.

## Step 4 — The continuation run page

- [x] `project_continuation_run_page(from_entry_id, request)` returns a
      `ForkPathPage` for the flat run beginning **at** `from_entry_id` and
      following preferred children to its leaf.
- [x] It reuses `project_lineage_page`, so positions, `continuation_count`,
      paging and truncation behave exactly as they do for the default path.
      That identity is the point: the nested list is the same component as the
      root list because it is the same projection.

## Step 5 — Protocol, commands and bindings

- [x] `ForkContinuationPageCommand` / `ForkContinuationPageSnapshot` and
      `ForkContinuationRunCommand`, following the existing path and branch
      commands exactly — protocol version, authority epoch, expected revision.
- [x] Register both in `longhorn-tauri-history-tree` with named re-exports,
      per Card 181 step 2.
- [x] Regenerate bindings and extend `ForkHistoryController` with
      `loadContinuations` and `loadContinuationRun`, each with the projection
      gap behaviour `loadBranches` already has.

## Acceptance

- [x] `effigy qa` passes, including `check:bindings`.
- [x] A projection test builds two forks at one entry and asserts
      `continuation_count` is 3 there and the continuations page lists all
      three, with exactly one `preferred`.
- [x] A projection test builds a fork off a fork and asserts each branch's
      divergence names its own parent branch, not the current one — the case
      the old computation collapsed.
- [x] A projection test asserts a continuation run page and a default path page
      of the same entries agree field for field, which is what lets the
      renderer recurse.
- [x] A projection test asserts a run's last entry has `continuation_count` 0.

## Evidence

- [x] The tests above, named in the batch log.

## Stop Conditions

- Stop if `continuation_count` cannot be computed without the page's own
  lineage. A count that depends on which page you asked for is a count a
  renderer cannot cache, and the badge would flicker between views.
- Stop if deriving a continuation's branch is not total — if any leaf turns out
  not to be a branch head, the picker has an unlabelled option and that is a
  modelling gap, not a display default.

## Continuation

Card 184 adds the activation half: choosing a continuation without walking into
it.

## Outcome — 2026-08-12

All five steps landed, with two deliberate departures from the card.

**The run reuses the path command.** Step 5 planned a parallel
`ForkContinuationRunCommand`. Instead `ForkPathTargetProjection` gained a
`Continuation { from_entry_id }` variant, so a nested run is literally the same
command and the same `ForkPathPageSnapshot` as the root list. That is the
identity the card asked for, expressed in the type system rather than asserted
by a test.

**Longest shared prefix alone does not find a parent.** Step 1's rule is
symmetric: a branch and the branch that forked off it share exactly the same
prefix, so each looks like the other's nearest ancestor, and the first test run
had `alternate` diverging from `deep`. The tiebreak is which run occupied the
fork point first -- when you fork at an entry, the continuation already there
was recorded earlier, so the parent is the candidate whose first divergent
entry has the lower sequence. A candidate whose lineage is a strict prefix of
ours has no divergent entry and is an ancestor outright.

Two existing assertions in `branch_and_path_pages_are_explicit_stable_and_hard_bounded`
had pinned artifacts of the old rule: `alternate` diverging at its own head
(the current branch was `alternate`, so the old code zipped it against itself)
and `main` diverging at `entry:b` (a fact about `alternate`, reported on
`main`). Both now read as the structure actually is -- `entry:b` off `main`,
and `None` for `main`, which forked off nothing.

Also fixed in passing: `assertForkNavigationCommand` accepted only `["kind"]`
for every target but `checkout`, so Card 181's `checkoutBranchRoot` would have
failed validation the first time a consumer sent one. Both target validators
now carry a per-variant field map.

The artifact proof was extended rather than stubbed: both packed consumers now
call the continuation surface and the trace they compare against the native
Rust run includes the anchor and its continuation ids.

Evidence:
- `one_entry_carries_every_continuation_it_has`,
  `a_fork_off_a_fork_diverges_from_its_own_parent`,
  `a_continuation_run_matches_the_path_page_for_the_same_entries`
  (`crates/longhorn-history-tree/tests/projection.rs`)
- `effigy qa` exit 0; the tree artifact proof moved 32 -> 39 tests.

## Follow-up — 2026-08-12, from the Poodle thread's b028 review

Two defects, both reported against the shipped card and both real.

**`root_continuation_count` reported the history root on every page.** True for
a default or branch path, false for a continuation run, where the page starts
mid-graph. It is now `preceding_continuation_count`, computed from the parent
of the lineage's first entry: the history root when the lineage starts at a
root, the anchor entry when it does not. One rule, true everywhere. Renamed
rather than patched, because a field called `root` that is not the root is the
kind of thing that survives a decade.

**The preferred-child invariant was a comment, not a guard.** Every forward
walk here follows preferred children and stops where there is none, so a node
with children and no preference truncates all of them — the run ends early and
a fork at its terminal entry cannot be opened.

Recording maintains the preference unconditionally, and pruning installs a
replacement, so neither is the hole. `ForkHistoryState::with_preferred_children`
is public and `from_state` never checked completeness. Two of this repo's own
fixtures were in exactly that state, including the Loophole-shaped graph in the
artifact proof, where the anchor carried the main chain and sixty-four
alternates with no preference between them. Nothing failed, because nothing
walked forward from the anchor.

The fix is in two halves, because the strict reading was wrong:

- A preference only means something among alternatives. With exactly one child
  there is nothing to choose, so `preferred_child_id` returns it whether or not
  a preference was recorded. Several fixtures relied on this without saying so.
- With two or more children a choice exists and must have been made.
  `from_state` rejects a state that has not, as `MissingPreferredChild`.

So the invariant a consumer can rely on is: a run's terminal entry has no
children at all. A renderer's fork count is therefore
`max(0, continuationCount - 1)` — the saturating form, because the plain
subtraction underflows to -1 at every terminal entry. The card and the Poodle
handoff both stated the unsaturated form.

Evidence: `a_state_whose_node_has_children_but_no_preference_is_rejected`, and
the root-choice row of `malformed_topology_rejection_matrix_is_deterministic`.
The tree artifact proof moved 39 -> 40 tests.
