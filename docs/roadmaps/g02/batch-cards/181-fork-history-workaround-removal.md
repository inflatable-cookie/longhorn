# 181 Fork History Workaround Removal

Status: ready
Owner: Tom
Roadmap: g02.016 batch 1
Governing refs: contract 011; contract 012; contract 017
Depends on: none
Blocks: Loophole deleting three pieces of workaround code
Auto-start next card: no

## Why

Loophole shipped fork history and reported five items from field use. Three are
workaround removals; this card takes them in the field's own priority order.
Each is verified against the code rather than taken from the report — one of
the five turned out to describe a defect Longhorn does not have, and that is
recorded on the milestone.

## Step 1 — Refresh the branches page alongside path

`packages/longhorn/src/history-tree/controller.ts`.

`refresh()` fetches a snapshot and a path, retries twice until they agree on a
revision, then installs both. `#branches` is never touched. `#changed()` calls
`refresh()`, so after any mutation the branches page a consumer already loaded
is pinned to an older revision, and `loadBranches()` throws
`ForkHistoryProjectionGapError` on the first mismatch with no retry of its own.
A consumer that loads branches once and keeps editing therefore holds a stale
page permanently: fork indicators stop rendering, current-branch lookups fail.

Loophole works around it with an event-driven reload plus a retry through the
gap. That is the controller's job.

- [ ] `refresh()` reloads the branches page when one is already loaded, inside
      the same attempt loop, so snapshot, path and branches install together on
      one revision or fail together.
- [ ] Do not fetch branches for a consumer that never asked. `#branches`
      undefined stays undefined.
- [ ] Preserve the loaded offset across the refresh; a consumer paged to offset
      40 does not silently jump to 0.
- [ ] `loadBranches()` keeps its gap error for the explicit-call case, where a
      caller asked for a specific offset against a revision that has moved.

## Step 2 — One re-export style across the Tauri crates

The request asked for `pub use commands::*` in `longhorn-tauri-history-tree` to
match notifications. The premise is wrong — see the milestone — but the
inconsistency it noticed is real and it misled a consumer.

`longhorn-tauri-notifications` is the only crate of ten that globs;
`longhorn-tauri-bridge`, `-command`, `-config`, `-history`, `-history-tree`,
`-operation`, `-settings` and `-transfer` all re-export named items.

- [ ] Pick one style and apply it to all ten. Named is the majority and states
      the surface explicitly; the glob's only advantage is making an unqualified
      `generate_handler!` import work, which the qualified path already does.
- [ ] Whichever is chosen, say so once where a host integrator will read it —
      the crate docs or the getting-started guide — with the qualified-path form
      spelled out, because that is the part a consumer got wrong.
- [ ] No compatibility re-export for the crate that changes. Pre-1.0.

## Step 3 — Checkout a branch root

`crates/longhorn-history-tree/src/navigation/types.rs:16`.
`ForkNavigationTarget::Checkout { branch_id, entry_id }` requires an entry.
An empty branch head — a nascent main — and a root-only switch have no entry to
name, so consumers special-case `AlreadyAtTarget` and `UnknownTarget` to get
there.

- [ ] Express the branch-root target. The request offers optional `entry_id` or
      a separate `CheckoutBranchRoot { branch_id }`; prefer the separate variant
      unless the planning shows otherwise, because an optional field makes every
      match site handle a combination that is only meaningful for one of them.
- [ ] Carry it through the navigation plan, receipt, rejection projection and
      the generated TypeScript.
- [ ] A checkout to a branch with an empty head succeeds rather than reporting
      an unknown target.

## Acceptance

- [ ] Longhorn's `effigy qa` passes, including `check:bindings` — steps 2 and 3
      both change generated output.
- [ ] A controller test proves branches refresh with path: load branches,
      mutate, assert the page revision moved without an explicit reload.
- [ ] A controller test proves the offset survives a refresh.
- [ ] A navigation test checks out a branch whose head is empty.
- [ ] Loophole's three workarounds are deleted, not left unused — the request
      names them in `apps/desktop/src-tauri/history_host.rs` and
      `apps/desktop/src/renderer/history/hub.svelte.ts`.

## Evidence

- [ ] The controller tests above, named in the batch log.
- [ ] The diff that removes Loophole's workarounds, referenced by commit.

## Stop Conditions

- Stop if step 1 cannot keep the three pages on one revision without holding a
  lock across three round trips. Serialising a consumer's reads behind a
  refresh is worse than the stale page it fixes, and that trade is the
  operator's.
- Stop if step 3's variant choice implies a protocol break for the linear
  history domain. It should not — `ForkNavigationTarget` is fork-only — but the
  generated types are shared surface and a break there is a consumer break.

## Continuation

Card 182 is paused on a persistence question about `recorded_at`, and Card 183
on Poodle's client-side stitcher. Neither auto-starts. Return to the milestone's
planning checkpoint after this card.

## Notes

The report's priority was 2 > 1 > 4 > 3 > 5. This card keeps that order for the
three it takes. Item 1 moved from "blocking a release" to "worth doing for
consistency" once the verification showed hosts can register the shipped
commands today — which means Loophole can delete its wrappers before this card
lands, and should.
