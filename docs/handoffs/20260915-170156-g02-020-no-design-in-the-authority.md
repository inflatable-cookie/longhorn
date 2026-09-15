---
kind: northstar-handoff
title: "g02.020 — No design in the authority"
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-09-15
updated: 2026-09-15
base_required: pushed-main
queue_dispatch: northstar-queue
queue_approval: "Operator-confirmed direction in this conversation on 2026-09-15: unblock and finish the open g02 work, then cut the first release; dispatch through northstar-queue."
roadmap: docs/roadmaps/g02/020-no-design-in-the-authority.md
tags: [coordination, handoff, worker, pr, g02, release-sequence]
---

## What This Thread Was Doing

The Longhorn planning thread ran a Northstar refresh and recompiled g02.020. The
Poodle dependency it waited on resolved — Poodle's `SettingsShell` carries the
redesigned shell — and Longhorn's binding already moved to it (`ec226f5a`,
2026-08-13). This lane finishes the job: no bespoke CSS left in the authority.

## Why It Matters

`longhorn-poodle-svelte` binds Longhorn authorities to Poodle components. A
binding that ships CSS is deciding layout, which is Poodle's job. Five surfaces
still carry 112 lines of layout CSS; removing it restores the boundary and adds
the check that stops it coming back.

## Current State

- **Done:** Poodle's redesigned shell exists; Longhorn's `SettingsShell.svelte`
  binds it with no `<style>` block; the group-label fault and the duplicate
  close are fixed with tests.
- **Still open:** the five surfaces that still ship a `<style>` block, and the
  stage-3 check.
- **Active spec lane:** none.
- **Current task:** `docs/roadmaps/g02/020-no-design-in-the-authority.md`.
- **Canonical refs:** contracts 012, 013, 020.
- **Remaining continuation envelope:** none — this handoff covers the lane.
- **Lane budget / pause signal:** normal; stop on the task's stop conditions.
- **Required sibling worktree links:** none.
- **Key files:**
  - `/Users/tom/Dev/projects/longhorn/packages/longhorn-poodle-svelte/src/update/poodle/UpdateSettings.svelte` (6 CSS lines)
  - `/Users/tom/Dev/projects/longhorn/packages/longhorn-poodle-svelte/src/config/poodle/StorageSettingsPage.svelte` (22)
  - `/Users/tom/Dev/projects/longhorn/packages/longhorn-poodle-svelte/src/config/poodle/BackupSettingsPage.svelte` (30)
  - `/Users/tom/Dev/projects/longhorn/packages/longhorn-poodle-svelte/src/config/poodle/RestoreSettingsPage.svelte` (28)
  - `/Users/tom/Dev/projects/longhorn/packages/longhorn-poodle-svelte/src/commands/poodle/KeybindingSettings.svelte` (26)

## Boundaries

- **In scope:** strip the five `<style>` blocks by composing Poodle
  `Stack`/`Grid`; keep the content and behaviour; add the check that fails on
  any `<style>` block in `longhorn-poodle-svelte`.
- **Out of scope:** the eight bindings that already meet the standard, and any
  move of settings *content* to Poodle. Poodle must not learn what a storage
  profile is.
- **Repo constraints:** follow `/Users/tom/Dev/projects/longhorn/AGENTS.md`.
  Where Poodle lacks a primitive (text wrapping is the known gap), raise a
  papercut instead of reintroducing local CSS.

## Important Context

- **Planning lineage:** absorbed Card 192; step 1 completed 2026-08-12.
- **How the plan fits the system:** contract 013 makes Poodle the presentation
  authority; contract 012 makes a binding a binding.
- **Decisions:** the settings family only; the redesigned shell is adopted, not
  ported.
- **Open tensions:** the canonical acceptance still names a Soundcheck worked
  example and an "under a hundred lines" shell. The binding is currently 382
  lines because it holds session state that Poodle cannot know. If the
  Soundcheck example needs a consumer run outside this repository, record it as
  outstanding evidence rather than blocking the CSS removal.

### UI Design Brief

Not applicable as a new design. The experience target is settled by Poodle's
`SettingsShell` contract; this lane removes layout decisions from Longhorn and
composes the settled primitives. Do not invent a new presentation direction.

## Suggested Next Move

Read the canonical task, then take the smallest surface first
(`UpdateSettings.svelte`, 6 CSS lines) to establish the `Stack`/`Grid`
replacement pattern before the three config pages.

## Completion Protocol

You are the implementation worker for this lane. Work on the queue-owned branch,
not `main`.

1. Implement the acceptance criteria, keeping every non-layout behaviour the
   pages own.
2. Run the `longhorn-poodle-svelte` type/vitest selectors and the Svelte check,
   then `effigy qa`.
3. Open a PR from the queue-owned branch and report `ready_for_review` through
   the Queue callback helper in your environment.
4. Fix review findings on the same branch; the reviewer resumes against your
   exact head.
5. After merge, closeout publishes the terminal record, roadmap status, and
   log. Do not edit roadmap indexes yourself.

Unresolved risk: the Soundcheck worked-example evidence may need a consumer run;
record it plainly if it does.
