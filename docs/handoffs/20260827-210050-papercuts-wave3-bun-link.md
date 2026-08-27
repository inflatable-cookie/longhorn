---
title: Papercuts wave 3 Bun link worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom / papercuts orchestrator
created: 2026-08-27
updated: 2026-08-27
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260827-210050-papercuts-wave3-bun-link.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
---

## What This Thread Was Doing

Wave 2 closed the Effigy `--` / rhai consumer copies and Bun-before-ts.
Two new Longhorn papercuts remain around `deps link bun` and greenfield
proofs ignoring a healthy Poodle link. `.agents.local.env` has no
gitignore entry. The operator approved papercuts wave 3.

You are the Longhorn implementation worker. `deps link bun` refusing
registry symlinks is an Effigy behavior — fix it here only if Longhorn
can wrap it; otherwise stop and report it as an Effigy follow-up rather
than inventing a local shim.

## Why It Matters

Fresh worktrees still cannot link Poodle over Bun's installed package
symlinks, then ignore the link they finally get.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `135acc853b8e910b62a6d878efca8022f5696986`
- **Pushed main verification:** local `HEAD` and `origin/main` both resolved
  to that SHA before this handoff was created.
- **Planning checkout:** clean before this handoff file was created.
- **Worker mode:** implementation worker dispatched by the orchestrator.
- **Worker branch:** `worker/papercuts-wave3-bun-link`
- **Worker worktree:** launcher worktree first. `.agents.local.env` was
  absent; ask before creating a manual fallback. Never use `/tmp`.
- **Ready work items, in order:**
  1. `deps link bun` refuses Bun registry package symlinks
  2. Greenfield proof ignores healthy Poodle link state
  3. `.agents.local.env` is a convention with no gitignore entry
- **Out of scope:** release-gate name order (Effigy); URL primitive
  duplication (wait for a third caller).
- **Canonical refs:** `PAPERCUTS.md`; `effigy deps link bun`;
  `.gitignore`; greenfield QA proofs that still want `POODLE_REPO`.
- **Required validation:** `deps link bun` can replace Bun's installed
  package symlink, or the remaining Effigy refusal is reported without a
  fake local wrapper; proofs consume the healthy link; `.agents.local.env`
  is gitignored. Fix the invalid papercut heading diagnostic if you touch
  the file.
- **PR URL:** pending
- **Merge authorisation:** absent; do not merge

## Boundaries

- If item 1 is purely Effigy, stop after proving it and file/keep the
  papercut pointed at Effigy. Do not start a nested Effigy worker.
- Item 2 can be a Longhorn proof change even if item 1 stays upstream.
- Do not merge.

## Important Context

- Wave 2 already required bun install before `check:ts`. This lane is the
  link/replace and proof-variable remainder.
- **Report to:** the operator.

## Suggested Next Move

Read this file, run the worktree preflight, then reproduce `deps link bun`
against a registry symlink.

## Completion Protocol

### Before you start

1. Read this handoff. Run the four git identity commands.
2. Accept a clean dedicated non-`main` registered worktree.
3. Confirm `HEAD == origin/main` and ancestor
   `135acc853b8e910b62a6d878efca8022f5696986`.
4. Confirm this handoff exists in `HEAD`.

### When the assigned runway is complete

1. Update `PAPERCUTS.md`. Push a PR. Do not merge.

### Review and merge path

Awaiting orchestrator review. Merge is operator-authorised only.

- **Closeout refs:** `PAPERCUTS.md`; this handoff; the PR.

### Handoff closeout

Gitignore `.agents.local.env` even if the link items stay blocked.
