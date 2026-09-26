---
title: Longhorn g02.049 release version sweep worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-09-26
updated: 2026-09-26
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260926-013602-g02-049-release-version-sweep-worker.md
base_required: pushed-main
queue_dispatch: northstar-queue
queue_approval: "Tom chose the picker follow-up and release tooling lanes on 2026-09-26 ('Let's do 1 and 2 now'), confirmed Longhorn-only release tooling with Effigy gaps left as upstream notes, and directed Chatterbox to dispatch through Northstar Queue."
tags: [coordination, handoff, worker, release-tooling]
---

## What This Thread Was Doing

The `0.2.1` release needed a hand-made version batch over 35 files because
`effigy release prepare` cannot see the workspace-excluded prototype locks and
version literals are spread across the proofs. This worker makes the next bump
one repo-owned command under
[g02.049](../roadmaps/g02/049-release-version-sweep.md). Read the card first.

## Why It Matters

Every Longhorn release currently costs a Queue lane just to bump versions, and
a missed literal only shows up as a proof failure. `release:gates` also runs
three of the six gates it implies, which let a drift reach CI once.

## Current State

- `main` carries the 2026-09-26 planning commit with this card.
- PR #35 (`cdd26dbf`) is the reference diff for every surface a bump touches.
- `effigy sync:prototype-locks` (`scripts/sync-prototype-locks.ts`) already
  rewrites Longhorn path versions in locks surgically.
- `.github/workflows/release.yml` runs `effigy qa`, then
  `effigy release:gates`.
- g02.047 runs in parallel on agent-control paths; overlap in `scripts/` is
  limited to the release-absence script, and the later PR rebases.

## Boundaries

- Own the paths in the card's dispatch manifest. Work only in the
  Queue-provided non-main worktree.
- No Effigy or `.github/workflows/` edits. Do not weaken any gate or
  `--locked`. Do not commit a version change; the scratch `0.2.2` bump is
  evidence only.
- No tag, publish, or workflow dispatch. Do not edit the card's lifecycle
  block or closeout surfaces.
- UI design brief: not applicable.

## Important Context

- `config/release.toml` comments assert some gate lines verbatim
  (`verify-private-candidate-docs-card127.ts`); keep those lines intact.
- Historical logs, evidence files, and changelog entries keep their literal
  versions.
- The `0.2.1` log and `PAPERCUTS.md` release entries describe the known Effigy
  gaps; record them as upstream notes, not fixes.

## Suggested Next Move

Diff PR #35 to list every version surface. Move the proofs and tests to one
version source, write `release:bump`, align `release:gates` with a guard, and
rewrite the runbook. Prove the bump in a scratch worktree with `effigy qa` and
`check:prototypes`, then discard it. Run `effigy qa` on the PR head and open a
PR for Queue's independent reviewer.

## Completion Protocol

Queue owns review, revision, and merge. Record changed paths, commands, the
scratch-bump evidence, and residual limits in the PR. Queue closes out g02.049
after merge.

Disposition trigger: transient worker handoff; prune through the normal
Northstar lifecycle after Queue closeout.
