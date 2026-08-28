---
title: Papercuts wave 5 Bun link and release-gate closeout worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-for-review
owner: Tom / papercuts orchestrator
created: 2026-08-28
updated: 2026-08-28
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260828-164810-papercuts-wave5-link-gates.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
---

## What This Thread Was Doing

Wave 3 proved `deps link bun` is an Effigy surface and left it open.
Effigy PR 48 now treats Bun registry-package symlinks as replaceable and
runs `[release.gates]` in declaration order. This repo still lists both
copies.

You are the Longhorn implementation worker. Prove those two behaviours
against current Effigy and close the copies. Do not rename gates. Do not
invent a local link shim.

## Why It Matters

Fresh worktrees still look blocked on a link conflict and a name-sorted
MSRV floor that Effigy already fixed.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `61beb470a330c78e841f9f069d2cb36a2a86d73d`
- **Worker mode:** implementation worker dispatched by the orchestrator.
- **Worker branch:** `t3code/link-gates-papercuts-wave5`
- **Worker worktree:** `/Users/tom/.t3/worktrees/longhorn/t3code-a1d5be69`
- **Item outcomes:**
  1. `deps link bun` refuses Bun registry package symlinks — **closed**.
     PATH Effigy `v0.12.1+local.9b9a3ba` (contains `02100eef` / PR 48)
     replaced the registry symlink under
     `node_modules/.bun/@inflatable-cookie+poodle-core@…` after
     `bun install` with no manual unlink.
  2. Release gates execute in name order — **closed**. Same PATH Effigy
     ran a fixture in declaration order (`zzz-first`, `aaa-second`,
     `mmm-third`). Gate names unchanged; card-127 asserts stay intact.
     `config/release.toml` comment updated.
- **Out of scope left open:** Endpoint URL validation duplicated across
  capability crates (wait for a third caller).
- **Canonical refs:** `PAPERCUTS.md`; `config/release.toml`;
  `scripts/verify-private-candidate-docs-card127.ts`; sibling Effigy
  `02100eefdde17db64652b2b26317bb284c504d8e` (PR 48).
- **Required validation:** Effigy `v0.12.1+local.9b9a3ba`. Link proof on
  this worktree after `bun install`. Gate proof from
  `effigy release gates` order on a fixture, not a rename.
- **PR URL:** https://github.com/inflatable-cookie/longhorn/pull/12
- **Merge authorisation:** absent; do not merge

## Boundaries

- Prove against the current pin. Do not wrap `deps link bun` with a fake
  local shim. Do not rename release gates.
- Do not merge.

## Important Context

- Wave 3 already gitignored `.agents.local.env` and fixed the greenfield
  Poodle-link proof.
- **Report to:** the operator.

## Suggested Next Move

Awaiting orchestrator review. Merge is operator-authorised only.

## Completion Protocol

### Before you start

1. Read this handoff. Run `git rev-parse --show-toplevel`,
   `git branch --show-current`, `git status --porcelain`, and
   `git worktree list --porcelain`.
2. Accept a clean dedicated non-`main` registered worktree. Record the
   actual path/branch.
3. Confirm `HEAD == origin/main` and ancestor
   `61beb470a330c78e841f9f069d2cb36a2a86d73d`.
4. Confirm this handoff exists in `HEAD`.

### When the assigned runway is complete

1. Update `PAPERCUTS.md`. Push a PR. Do not merge.

### Review and merge path

Awaiting orchestrator review. Merge is operator-authorised only.

- **Closeout refs:** `PAPERCUTS.md`; this handoff; the PR.

### Handoff closeout

If PATH `effigy` is older than PR 48, keep both copies open and report
the version. Leave the URL-primitive papercut open.
