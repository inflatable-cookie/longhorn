---
title: Papercuts wave 13 agent-control dead-pid sweep worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom / papercuts orchestrator
created: 2026-08-29
updated: 2026-08-29
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260829-231910-papercuts-wave13-dead-pid.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
---

## What This Thread Was Doing

Figmatic filed that `find-instance.ts` skipped 62 stale
`com.inflatablecookie.figmatic-*.json` files before two live ones.
Discovery only drops those files on clean exit, not when the pid is
already dead.

You are the Longhorn implementation worker. Sweep discovery files whose
pid is dead on mount (and on lookup if that is the same code path). Do
not edit Figmatic. Leave the URL-primitive papercut and the AGENTS-audit
/ jetstream-workspace items for later.

## Why It Matters

Instance discovery is slow and the directory is unreadable by hand when
choosing between live Figmatic instances.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `7d6a1d8ad644af815e1bbec55fa5ec1588700b52`
- **Pushed main verification:** local `HEAD` and `origin/main` both resolved
  to that SHA before this handoff was created.
- **Planning checkout:** clean before this handoff file was created.
- **Worker mode:** implementation worker dispatched by the orchestrator.
- **Worker branch:** `worker/papercuts-wave13-dead-pid`
- **Worker worktree:** launcher first. `.agents.local.env` was absent;
  ask before creating a manual fallback. Never use `/tmp`.
- **Required sibling worktree links:** none
- **Ready work items, in order:**
  1. Longhorn agent-control discovery directory keeps dead-pid files —
     remove (or ignore and then unlink) discovery JSON whose pid is not
     running, on mount / find-instance, not only on clean process exit.
     Add-and-close this papercut in *this* repo's `PAPERCUTS.md`; the
     filing lives in Figmatic
- **Out of scope:** Endpoint URL duplication (wait for a third caller);
  consumer AGENTS audit selector; rust-audit jetstream workspace
  membership; editing Figmatic; GitHub workflows.
- **Canonical refs:** Figmatic `PAPERCUTS.md` dead-pid entry;
  Longhorn agent-control discovery under
  `~/Library/Application Support/longhorn/state/agent-control`.
- **Required validation:** a focused test that a discovery file with a
  dead pid is not returned and is unlinked (or equivalently gone from
  the directory). Do not require a live Figmatic window.
- **PR URL:** pending
- **Merge authorisation:** absent; do not merge

## Boundaries

- Discovery sweep only. Do not change the live instance protocol.
- Do not merge.

## Important Context

- Filed from Figmatic during 016-12 baseline runs. Fix the Longhorn
  surface; Figmatic can close its copy later.
- **Report to:** the operator.

## Suggested Next Move

Read this file from the top. Run the worktree-safety preflight. After
the committed `HEAD` handoff checks out, find the discovery writer and
sweep dead pids on mount.

## Completion Protocol

### Before you start

1. Read this handoff path. Its `worker_mode: implementation` and
   `dispatch_authority: orchestrator` metadata activate worker mode. Then
   run `git rev-parse --show-toplevel`, `git branch --show-current`,
   `git status --porcelain`, and `git worktree list --porcelain`.
2. If the current root is a registered worktree, status is empty, and the
   branch is not `main`, accept it. Record the actual path/branch.
3. If the launcher supplied a dirty or `main` worktree, stop and report
   it. `.agents.local.env` was absent; ask before creating a fallback.
   Never use `/tmp`.
4. From the selected worktree, record the repository-relative path
   `docs/handoffs/20260829-231910-papercuts-wave13-dead-pid.md`.
   Confirm `HEAD == origin/main`, ancestor
   `7d6a1d8ad644af815e1bbec55fa5ec1588700b52`, and that relative path in
   `HEAD`. Load
   `git show HEAD:docs/handoffs/20260829-231910-papercuts-wave13-dead-pid.md`.
   If the absolute dispatch file differs, stop. The `HEAD` copy is
   canonical.
5. Required sibling list is `none`. Skip link setup.
6. Read `AGENTS.md` and `PAPERCUTS.md`.

### When the assigned runway is complete

1. Add-and-close the papercut in this repo's `PAPERCUTS.md`.
2. Push a PR. Do not merge.

### Review and merge path

Awaiting orchestrator review. Merge is operator-authorised only.

- **Closeout refs:** `PAPERCUTS.md`; this handoff; the PR.

### Handoff closeout

Leave URL-primitive, AGENTS-audit, and jetstream-workspace items open.
