---
title: Papercuts wave 2 closeout and QA worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom / papercuts orchestrator
created: 2026-08-27
updated: 2026-08-27
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260827-181250-papercuts-wave2-closeout-qa.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
---

## What This Thread Was Doing

Effigy wave 1 (PR 45) closed Doctor inline `{ rhai = ... }`, built-in
`docs`, and global `--` after `--`. Longhorn's `PAPERCUTS.md` still lists
the first two as open. Fresh worktree QA still reaches TypeScript without
`bun install`, and prototype lockfiles still go stale.

You are the Longhorn implementation worker for this lane. Do not re-fix
Effigy. Verify the upstream fix, close the consumer copies, and land the
two Longhorn-local items.

## Why It Matters

Doctor and `--repo` papercuts in this file send agents to re-work a
closed Effigy bug. Fresh worktrees still drown in TypeScript noise.
Prototype `Cargo.lock` files fail release gates days later.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `c94f72e9d361175c61a97fe867ba7b63f58a2db8`
- **Pushed main verification:** local `HEAD` and `origin/main` both resolved
  to that SHA before this handoff was created.
- **Planning checkout:** clean before this handoff file was created.
- **Worker mode:** implementation worker dispatched by the orchestrator.
- **Planning artifacts included at the base:** `PAPERCUTS.md`; this handoff.
- **Worker branch:** `worker/papercuts-wave2-closeout-qa`
- **Worker worktree:** use the launcher worktree. This handoff does not
  select a manual fallback path.
- **Manual fallback command:** only after the operator supplies
  `AGENTS_WORKTREE_CONTAINER_DIR`. `.agents.local.env` was absent.
- **Active spec lane:** none.
- **Roadmap milestone:** none.
- **Ready work items, in order:**
  1. Close the Doctor inline-rhai papercut if current Effigy admits the
     shape; otherwise document the remaining gap
  2. Close the global `--` after `--` papercut if current Effigy ends
     global flag parsing; otherwise document the remaining gap
  3. Fresh worktree QA reaches TypeScript without installing dependencies
  4. Prototype lockfiles go stale when a workspace crate gains a
     dependency
- **Allowed runway:** those four items only, one PR.
- **Remaining card budget:** four papercuts.
- **Dispatch topology:** serial inside Longhorn; parallel with other
  wave-2 repos.
- **Parallel safety check:** no shared files with the Effigy wave-2
  worker. Do not edit Effigy.
- **Canonical refs:** `AGENTS.md`; `PAPERCUTS.md`; `effigy.toml`;
  `prototypes/*/Cargo.lock`; `check:ts` / `check:prototypes`.
- **Model capability profile:** capable coding model, medium reasoning.
- **Tool/runtime restrictions:** do not edit Effigy; do not rename
  release gates as a sort hack (`verify-private-candidate-docs-card127.ts`
  asserts gate lines verbatim). Release-gate *ordering* stays an Effigy
  item, out of this lane.
- **Required validation:** `effigy doctor` no longer flags the inline
  rhai shape, or the entry stays open with a measured remaining gap;
  `effigy <task> -- --repo …` behaviour matches the closed Effigy
  papercut, or the entry stays open; `effigy qa` / `check:ts` in a
  no-`node_modules` worktree either installs or fails with the install
  selector; a note or check that prototype lockfiles update with
  workspace deps. Do not require a full release-gates run unless cheap.
- **PR base/head:** current pushed `main` / selected worker branch
- **PR URL:** pending
- **Review state:** awaiting orchestrator review after worker completion
- **Merge authorisation:** absent; do not merge

## Boundaries

- **In scope:** close or re-evidence the two Effigy-fixed papercuts;
  require or perform locked Bun bootstrap before TypeScript checks;
  teach the prototype lockfile step (check or card note).
- **Out of scope:** release-gate execution order; URL primitive
  duplication across licence/update (explicitly wait for a third
  caller); `deps link bun` replacing registry symlinks unless that is
  required for item 3.
- `--` closeout: prove against the Effigy currently on PATH / the
  consumer pin, not against folklore.
- QA: make TypeScript checks depend on `bun install`, or fail with that
  selector instead of `bun x tsc` fetching a compiler into a vacuum.
- Prototype locks: `check:prototypes` in a cheap lane, or a lockfile-sync
  note on the dependency-sweep card. Prefer a check if one already
  exists and is unused.
- Also fix the invalid papercut heading on line 201 if you touch
  `PAPERCUTS.md` (collection diagnostic).
- Do not merge the PR.

## Important Context

- **Planning lineage:** papercuts wave 2. Effigy PR 45 already closed the
  CLI/doctor items in the Effigy file.
- **Report after:** rhai/doctor proof; `--` proof; bun-before-ts; prototype
  lockfiles; then PR.
- **Report to:** the operator, who will relay progress to the orchestrator.

## Suggested Next Move

Read this file from the top. Run the worktree-safety preflight. Use the
launcher worktree if it is clean, dedicated, and not `main`.

Start by running `effigy doctor` against the inline rhai task shape.

## Completion Protocol

### Before you start

1. Read this handoff. Then run `git rev-parse --show-toplevel`,
   `git branch --show-current`, `git status --porcelain`, and
   `git worktree list --porcelain`.
2. If the current root is a registered worktree, status is empty, and the
   branch is not `main`, accept it.
3. If the launcher supplied a dirty or `main` worktree, stop and report
   it. `.agents.local.env` was absent, so ask before creating a fallback.
   Never use `/tmp`.
4. Confirm `HEAD == origin/main`, confirm
   `git merge-base --is-ancestor c94f72e9d361175c61a97fe867ba7b63f58a2db8 HEAD`,
   and confirm this handoff exists in `HEAD`.
5. Read `AGENTS.md` and `PAPERCUTS.md`.

### While you work

- Commit in meaningful chunks.
- Report through the operator after each item.

### When the assigned runway is complete

1. Run the validation named above.
2. Close finished papercuts in `PAPERCUTS.md`.
3. Push the worker branch and open a PR against current pushed `main`.
4. Report the PR URL. Do not merge.

### Review and merge path

Awaiting orchestrator review. Merge is operator-authorised only.

- **Closeout refs:** `PAPERCUTS.md`; this handoff; the PR.

### Handoff closeout

If doctor still flags inline rhai on the current Effigy, keep that
papercut open and say so. Do not start an Effigy worker from this repo.
