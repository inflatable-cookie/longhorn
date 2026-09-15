---
kind: northstar-handoff
title: "g02.037 — Focused panel surfaces"
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
tags: [coordination, handoff, worker, pr, g02, release-sequence]
---

## What This Thread Was Doing

The Longhorn planning thread ran a Northstar refresh, recompiled the open g02
lanes, and set a first-release sequence. This is one of three parallel lanes in
that sequence: it completes the focused-panel Surface property that Loophole
currently rebuilds by convention.

## Why It Matters

A Surface that presents one panel full-surface is a real consumer need —
Loophole holds an app-side `focusSurfacePanels` map, a derived `focusPanel`, and
a drop guard. Three unrelated mechanisms stand in for one missing property, and
none of them survives a second consumer. Putting the property in the Surface
model deletes the convention.

## Current State

- **Done:** the task is planned and ready; its decisions and six Work steps are
  written in the canonical task.
- **Still open:** all implementation. `SurfaceRecord` does not yet carry
  presentation, and there is no `SetSurfacePresentation` command.
- **Active spec lane:** none.
- **Current task:** `docs/roadmaps/g02/037-focused-panel-surfaces.md`.
- **Canonical refs:** contracts 002 and 014; contract 014 is superseded by 002
  for the layout boundary.
- **Remaining continuation envelope:** none — this handoff covers the lane.
- **Lane budget / pause signal:** normal; stop only on the task's stop
  conditions.
- **Required sibling worktree links:** none.
- **Key files:**
  - `/Users/tom/Dev/projects/longhorn/docs/roadmaps/g02/037-focused-panel-surfaces.md`
  - `/Users/tom/Dev/projects/longhorn/crates/longhorn-surfaces/src/`
  - `/Users/tom/Dev/projects/longhorn/packages/longhorn/src/surfaces/`

## Boundaries

- **In scope:** the task's six Work steps — `SurfacePresentation`,
  `SetSurfacePresentation`, validation, regenerated bindings, conformance
  tests, and one documented consumer obligation.
- **Out of scope:** policing container contents from the surfaces crate, and
  widening `LayoutContainerInventory`. The container invariant stays a
  documented consumer obligation until a composition-layer owner exists.
- **Repo constraints:** follow `/Users/tom/Dev/projects/longhorn/AGENTS.md`; run
  the narrowest Effigy selectors while working, then `effigy qa` before the PR.

## Important Context

- **Planning lineage:** promoted from Card 177 during the flattened-task
  migration. The g02 runway owns sequencing.
- **Decisions:** presentation is a Surface property, not a panel claim; the
  field defaults to `regional` so a pre-change document loads unchanged;
  surface transfer is unaffected because the guard belongs to panel transfer.
- **Open tensions:** the container invariant has no composition-layer owner
  yet. Do not invent one or enforce it from `longhorn-surfaces`.

### UI Design Brief

Not applicable. This is a data-model and protocol change, not a UI delivery.

## Suggested Next Move

Read the canonical task end to end, then implement Work step 1
(`SurfacePresentation` in `longhorn-surfaces`) and follow the steps in order.
Keep the typed rejections and the snapshot → mutate → snapshot round-trip
exactly as the acceptance states.

## Completion Protocol

You are the implementation worker for this lane. Work on the queue-owned branch,
not `main`.

1. Implement the task's acceptance criteria.
2. Run the focused `longhorn-surfaces` selectors and `check:bindings`, then
   `effigy qa`.
3. Open a PR from the queue-owned branch and report `ready_for_review` through
   the Queue callback helper in your environment.
4. Fix review findings on the same branch; the reviewer resumes against your
   exact head.
5. After merge, closeout publishes the terminal record, roadmap status, and
   log. Do not edit roadmap indexes yourself.

Unresolved risk: the container invariant stays a documented consumer
obligation. If a contract contradiction with 002 or 014 appears, stop and
report it.
