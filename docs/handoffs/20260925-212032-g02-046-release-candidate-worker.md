---
title: Longhorn g02.046 0.2.1 release candidate worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-09-25
updated: 2026-09-25
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260925-212032-g02-046-release-candidate-worker.md
base_required: pushed-main
queue_dispatch: northstar-queue
queue_approval: "Tom authorised carrying g02.045 through Northstar Queue delivery and release on 2026-09-24, chose 0.2.1, authorised its preparation and publication on 2026-09-25, and said 'Go for it' after the excluded prototype lock gate blocked preparation."
tags: [coordination, handoff, worker, release-candidate]
---

## What This Thread Was Doing

Longhorn g02.045 is merged and Figmatic proved the source-linked New project
selection flow. This worker prepares the reviewed `0.2.1` candidate under
[g02.046](../roadmaps/g02/046-agent-selection-release-candidate.md). Read that
card and contract 012 before editing. The release owner will run the final
gates, tag and npm publish after Queue merges this candidate.

## Why It Matters

Figmatic g01.052 is held before PR because npm `0.2.0` and Rust tag `v0.2.0`
predate the agent-answerable selection API. An exact published `0.2.1` identity
will let the retained Figmatic worker replace source links and continue.

## Current State

- Longhorn `main` is clean, pushed at `d159d272042d190c74bf4b9a841667712172d427`.
- CI run `36186070892` passed all four jobs on that SHA. `effigy release
  simulate` and `release status --check-gates` each passed seven gates before
  the version bump.
- `effigy release prepare --yes --check-gates` temporarily bumped to `0.2.1`,
  then failed `check:prototypes --locked` on `gpui-composition/Cargo.lock`.
  Effigy rolled back. No `.release-prepared.json`, tag, or npm publish exists.
- `PAPERCUTS.md` records this exact gap from the `0.2.0` release. Its earlier
  release used a manual coordinated version batch. The eight excluded
  prototype locks must be updated before the locked gate can pass.
- Current task: `g02.046` at
  `docs/roadmaps/g02/046-agent-selection-release-candidate.md`. No other
  Longhorn worker owns these version files.

## Boundaries

- Own the Longhorn `0.2.1` candidate version surfaces and a narrow repeatable
  excluded-lock sync procedure. The task card names the complete acceptance
  and stop conditions. Work only in the Queue-provided non-main worktree.
- Keep third-party lock entries stable. Do not weaken `--locked`, change
  runtime behavior, edit `.github/workflows/`, edit Effigy or Figmatic, tag,
  publish, or dispatch GitHub workflows.
- Use the root `AGENTS.md`, Effigy selectors, and the current pushed handoff.
  Preserve unrelated changes. If the launcher workspace is dirty or the
  handoff does not match its committed `HEAD`, stop.
- UI design brief: not applicable.

## Important Context

- Contract 012 requires coordinated Rust/npm Longhorn versions. Rust crates
  remain `publish = false` and consumers take them by git tag; three npm
  packages publish from `release.yml`.
- `effigy release prepare` syncs the root `Cargo.lock` but cannot sync the
  excluded prototype locks in the installed release tool. Do not repeat that
  known failing prepare command in this worker lane.
- The active proof scripts, adapter peer declarations, skill stamp and API
  reference also contain version expectations. Search by Longhorn identity;
  do not mass-replace historical `0.2.0` evidence or third-party versions.
- Review the `0.2.0` publication log for the prior candidate batch shape.
  This candidate needs independent review and merge before the release owner
  runs exact-commit CI and the full seven gates again.

## Suggested Next Move

Read the task card, contract 012, `PAPERCUTS.md` release entries, and `0.2.0`
publication log. Inspect every active version surface, prepare one coherent
`0.2.1` candidate diff, update each prototype lock through Cargo, and verify
that third-party lock entries did not move. Run focused version/lock checks,
`effigy check:prototypes`, `effigy qa`, and `git diff --check`. Push a PR for
Queue's independent reviewer. Report the exact candidate SHA and any gate
limit; do not tag or publish.

## Completion Protocol

Queue owns review, revision and merge. Record the candidate's changed paths,
commands, lock-diff assessment, validation results and residual limits in the
PR. Let Queue close out g02.046 after merge. The release owner then verifies
the exact merged SHA and performs the operator-authorised tag/npm sequence;
Figmatic receives only the actual published npm version and Rust tag/commit.

Disposition trigger: transient worker handoff; prune through the normal
Northstar lifecycle after Queue closeout.
