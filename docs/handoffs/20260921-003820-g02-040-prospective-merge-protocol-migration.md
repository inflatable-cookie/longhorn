---
title: g02.040 — Prospective-merge protocol migration
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
status: ready-to-launch
owner: Tom
created: 2026-09-21
updated: 2026-09-21
base_required: pushed-main
roadmap: docs/roadmaps/g02/040-prospective-merge-protocol-migration.md
queue_dispatch: northstar-queue
queue_approval: "Tom authorized the Northstar v4 consumer rollout on 2026-09-16 with 'Go for it' (Northstar g03.020 ready-state rubric); this lane continues the census rollout dispatched by the Northstar Chatterbox."
queue:
  capability: mechanical
  skipPRReview: false
---

## What This Thread Was Doing

Apply Northstar g03.020's accepted Queue manifest migration to Longhorn.

## Why It Matters

The prospective merge target checks the tree Queue will actually merge. A newer
base change outside the lane no longer produces a false reviewed-head refusal,
so Longhorn's lanes stop stalling on stale-branch comparisons.

## Current State

Longhorn main is clean and synchronized at the planning commit containing this
handoff. The installed migration dry-run accepts the exact v3 manifest. Required
sibling worktree links: none. Use the automatic mechanical/general pool;
frontier-worker justification: none.

## Boundaries

Change only `.paseo/queue.json` through the installed command. Do not change
product code or planning, dependencies, releases, CI, Queue state, threads or
workspaces.

## Important Context

Run `effigy skill run northstar/lifecycle:migrate-premerge` first, then the same
selector with `-- --write`. The only accepted edit is v3 to v4 plus
`reviewed_head` to `prospective_merge`. Divergent manifests fail closed; do not
hand-edit around a refusal.

## Suggested Next Move

Verify the clean branch and the committed handoff, run dry-run and write mode,
then inspect the complete diff before validation.

## Completion Protocol

Prove `applied`, then `unchanged` on replay, validate the v4 manifest, run the
focused repository checks and `git diff --check`. Commit and push one non-draft
PR and report `ready_for_review` through the authenticated Queue callback.
Independent review uses another provider/model identity. Do not merge, refresh
an installed skill, mutate other repositories or publish a release.
