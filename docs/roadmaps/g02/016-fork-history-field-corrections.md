# g02.016 Fork History Field Corrections

Status: ready
Owner: Tom
Governing refs: contract 011; contract 012; contract 017
Depends on: none

## Outcome

Loophole shipped fork history end-to-end — pulse-history on
`longhorn-history-tree`, HistoryCentre bound through `longhorn-tauri-history-tree`
and `longhorn-poodle-svelte` — and sent back five items from real use. This
milestone takes them: three remove workaround code standing in Loophole today,
two extend the surface for the HistoryCentre v2 redesign.

The value is the provenance. These are not inferred gaps; each one cost a
consumer something specific, and three of them delete live code the release
they land.

## What The Request Got Wrong

Verified against the code on 2026-08-12 before compiling anything. Two of the
five need correcting, and one is wider than reported.

**Item 1 is not a Longhorn defect.** The request says `longhorn-tauri-history-tree`
re-exports named functions only, so "every host must write wrapper commands".
Hosts do not. `#[tauri::command]` on a `pub fn` emits `#[macro_export]` on the
generated `__cmd__` macro (`tauri-macros-2.5.5/src/command/wrapper.rs:142`),
which places it at the crate root whatever the module's visibility. So
`tauri::generate_handler![longhorn_tauri_history_tree::longhorn_history_tree_snapshot]`
resolves both halves today. Nucleus proves it: it registers
`longhorn_tauri_command::longhorn_command_catalogue` directly from a crate with
the identical shape — private `mod commands`, named re-export — and compiles.

Loophole's wrappers can be deleted now, without waiting for a release. That
also removes the name drift that cost the debugging session, because the names
stop being hand-written.

There is still something to fix, and it is the opposite of what was reported:
`longhorn-tauri-notifications` is the **outlier**. Eight of the ten Tauri
crates use named re-exports; only notifications globs. Consistency here is
worth having because the request shows what its absence costs — a consumer
compared two crates, inferred a rule from the wrong one, and paid for it.

**Item 4 is wider than stated.** `HistoryEntryMetadata` lives in
`longhorn-history` (`src/entry.rs:73`), not `longhorn-history-tree`. Adding
`recorded_at` touches the linear history domain, its persistence envelope and
its generated TypeScript as well as the fork tree's. That does not change
whether it is worth doing; it changes the size.

Items 2, 3 and 5 are confirmed as described.

## Planning Gaps

- ~~**Item 4 needs a persistence decision before it is ready.**~~ Closed
  2026-08-12: entries written before the field lands read back as `None`,
  permanently, and nothing backfills. Recorded on Card 182. The request said
  "structural migration path exists". Which migration, and whether an entry
  written before this lands reads back as `recorded_at: None` or fails the
  envelope, is not settled. That is operator policy about stored data, so the
  card stays paused until it is answered.
- ~~**Item 5 needs the client-side stitcher first.**~~ Closed 2026-08-12:
  Poodle has built HistoryCentre v2, so its stitcher is readable and Card 183
  can be designed against it rather than a guess.

## Execution Plan

- [x] **Batch 1. Remove the workarounds** (Card 181, complete 2026-08-12). Items 2, 1 and 3.
      `ForkHistoryController` refreshes its branches page alongside path;
      the Tauri crates agree on one re-export style; `Checkout` expresses a
      branch-root target. Ordered by the field's own priority.
- [x] **Batch 2. Host-supplied `recorded_at`** (Card 182, complete 2026-08-12). Optional, consumer-supplied, carried inert from
      `HistoryEntryMetadata` through the envelope to `ForkEntryRecord` and the
      generated types, in both history domains.
- [ ] **Batch 3. Topological tree projection** (Card 183, unblocked; ready to
      plan against Poodle's shipped stitcher). A single paged `ForkTreePage` in ancestry order with branch
      and lane annotations, moving the stitch into the authority.

## Goals

- [ ] Loophole deletes its branches-reload retry loop, its command wrappers and
      its checkout special-casing, and nothing replaces them.
- [ ] A consumer that loads branches once and keeps editing sees a current page
      without asking for one.
- [ ] The ten Tauri crates answer the same question the same way.
- [x] Version captions can show a time without the history crates owning a
      clock.

## Acceptance Criteria

- [ ] `ForkHistoryController.refresh()` leaves snapshot, path and branches on
      one revision, or fails as one.
- [ ] `loadBranches` no longer throws a projection gap on the first mismatch
      after a mutation.
- [ ] Every `longhorn-tauri-*` crate re-exports its commands the same way, and
      a test or doc states which way.
- [ ] A branch with an empty head can be checked out without the consumer
      special-casing `AlreadyAtTarget` or `UnknownTarget`.
- [ ] Loophole's workaround code is gone, not merely unused.

## Explicit Non-goals

- The history crates do not gain a clock. `recorded_at` is supplied or absent.
- The tree never reads `recorded_at`; ordering stays structural.
- No compatibility alias for the re-export change. Pre-1.0, per contract 001.

## Next Task

Batches 1 and 2 are complete and both planning gaps are closed. Card 183 is the
remaining work: read Poodle's HistoryCentre v2 stitcher, then compile the
topological `ForkTreePage` against what it actually consumes.

Loophole can delete all three workarounds now. The command wrappers never
needed a release — see item 1 above.

## Planning Checkpoint

Before Batch 3. Card 183 moves a stitch from a consumer into the authority, so
what the projection emits should be settled against Poodle's shipped stitcher
rather than designed ahead of it.
