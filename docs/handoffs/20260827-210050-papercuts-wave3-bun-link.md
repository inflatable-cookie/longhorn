---
title: Papercuts wave 3 Bun link worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-for-review
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
- **Worker mode:** implementation worker dispatched by the orchestrator.
- **Worker branch:** `worker/papercuts-wave3-bun-link`
- **Worker worktree:** `/Users/tom/.t3/worktrees/longhorn/t3code-99359bee`
- **Item outcomes:**
  1. `deps link bun` refuses registry symlinks — **Effigy follow-up**.
     Reproduced on Effigy v0.12.1; no Longhorn shim.
  2. Greenfield proof ignores healthy Poodle link — **fixed** in
     `scripts/verify-greenfield-card125.ts` (POODLE_REPO → healthy bun
     link → sibling fallback).
  3. `.agents.local.env` gitignore — **fixed**. Local file not created
     (ask-before-create still applies).
- **Out of scope:** release-gate name order (Effigy); URL primitive
  duplication (wait for a third caller).
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

Orchestrator review of the PR. Item 1 needs an Effigy change to replace
Bun registry package symlinks safely.

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
Done. Item 1 remains open and pointed at Effigy.
