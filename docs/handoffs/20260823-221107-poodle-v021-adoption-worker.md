---
title: Poodle 0.2.1 adoption worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: poodle-orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-08-23
updated: 2026-08-23
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260823-221107-poodle-v021-adoption-worker.md
base_required: pushed-main
tags: [coordination, handoff, worker, dependency-adoption]
---

## Job

Adopt Poodle `0.2.1` throughout Longhorn. This is Poodle runway card
`g16.002`, frozen at Poodle main commit `6d6379f2`:

`/Users/tom/Dev/projects/poodle/docs/roadmaps/g16/002-longhorn-poodle-v021-adoption.md`

Read that card first. It owns the outcome, scope, acceptance, and stop
conditions. Longhorn owns the implementation and evidence.

## Starting State

- Repository: `git@github.com:inflatable-cookie/longhorn.git`
- Planning branch: `main`
- Planning base before this handoff: `854845cf105246052145974e381a4590ea9b6cf9`
- Worker branch label: `worker/poodle-v021-adoption`
- Main was clean and equal to `origin/main` when this handoff was compiled.
- Published sources are confirmed: npm core/Svelte `0.2.1`, Poodle git tag
  `v0.2.1`, and Rust package versions `0.2.1`.
- Merge authority stays with the Poodle orchestrator. Open a PR; do not merge.

## Required Change

- Root Poodle web development dependencies: exact `0.2.1`.
- `@inflatable-cookie/longhorn-poodle-svelte` peer:
  `@inflatable-cookie/poodle-svelte` exact `0.2.1`.
- Every active example's direct Poodle core/Svelte dependency: exact `0.2.1`.
- `crates/longhorn-poodle` `poodle-specs`: version `0.2.1`, tag `v0.2.1`.
- Both GPUI prototypes: every direct Poodle git dependency on tag `v0.2.1`.
- Regenerate Bun and Cargo locks from those declarations. Review the lock diff
  and avoid unrelated upgrades.
- Make only bounded Longhorn compatibility fixes exposed by the new release.

Do not change Longhorn's own `0.1.0` package/crate versions. Do not mass-edit
historical prose or fixtures simply because they contain `0.1.0`. Search by
Poodle package/crate identity, not by bare version string.

## Boundaries

- No Poodle edits, local Poodle overrides, aliases, compatibility shims, or
  weakened peers.
- No Longhorn release, package version bump, or publication.
- Preserve Longhorn/Poodle ownership boundaries and contract 012.
- No visible or focus-taking application run. Use headless validation only.
- Stop if migration needs a public Longhorn API/wire decision, reveals a
  Poodle release defect, or causes material unrelated lockfile churn.

## Worktree Preflight

Use the clean dedicated non-`main` worktree supplied by the launcher. Before
broad reads, run:

```sh
git rev-parse --show-toplevel
git branch --show-current
git status --porcelain
git worktree list --porcelain
git fetch origin
```

Accept the launcher worktree even if its generated branch/path differs from
the label above. Do not create a second worktree, reset, clean, or stash. Stop
if the checkout is dirty, on `main`, or does not contain this handoff in
`HEAD`. Confirm `HEAD == origin/main`, then read `AGENTS.md`, this handoff, the
Poodle card, and the directly affected manifests.

## Validation And Handoff Back

Use `effigy tasks` to confirm selectors, then run the narrow package/example
and Rust checks relevant to the changed surfaces. Finish with `effigy qa` and
`git diff --check`. Do not run a release mutation.

Before opening the PR, prove mechanically that active Poodle declarations and
locks contain no `0.1.0`, all Poodle git dependencies use `v0.2.1`, and npm
resolution is registry-backed. Record changed manifests/locks, exact commands,
results, bounded compatibility work, and any residual historical matches in
the PR. Push the worker branch and open a PR to `main`; report its URL to the
operator and stop.
