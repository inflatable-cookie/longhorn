---
kind: northstar-handoff
title: "g02.041 — Staged update protocol"
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-09-22
updated: 2026-09-22
base_required: pushed-main
queue_dispatch: northstar-queue
queue_approval: "Operator-approved contract 018 amendment and task path on 2026-09-22: staged update protocol (g02.041) and exclusive admission lease (g02.042), released as Longhorn 0.2.0 for the acowtancy Desktop lane g05.179."
roadmap: docs/roadmaps/g02/041-staged-update-protocol.md
tags: [coordination, handoff, worker, pr, update-protocol]
---

## What This Thread Was Doing

A consumer blocker from the acowtancy Desktop lane (`g05.179`, contract 038)
showed a real gap: Longhorn 0.1.0's `UpdateController::install` fetches,
verifies, gates, and applies in one locked call, so a surface cannot show byte
progress or retain a downloaded update through "Later". Contract 018 was amended
on 2026-09-22 to require a staged protocol. This lane implements it.

## Why It Matters

The consumer needs Download → observable byte progress → Ready →
Restart/Later without weakening verification or building a parallel updater.
Staging is the missing mechanism; verification and gating must not move a
millimetre.

## Current State

- **Done:** contract 018 amended; this task and handoff are committed; the
  operator approved the amendment and the 0.2.0 release path.
- **Still open:** the protocol types, the controller split, the live-progress
  seam, the Tauri seam, the generated projections, and the tests.
- **Active spec lane:** none.
- **Current task:** `docs/roadmaps/g02/041-staged-update-protocol.md` (`g02.041`).
- **Canonical refs:** contract 018 (`Staged Download And Install`); Desktop
  contract 038.
- **Remaining continuation envelope:** `g02.042` (exclusive admission lease)
  follows this lane.
- **Lane budget / pause signal:** normal; stop on the task's stop conditions.
- **Required sibling worktree links:** none.
- **Key files:**
  - `crates/longhorn-update/src/protocol.rs`
  - `crates/longhorn-update/src/controller.rs`
  - `crates/longhorn-tauri-update/src/handler.rs`
  - `packages/longhorn/src/update/controller.ts`

## Boundaries

- **In scope:** the staged protocol — prepare/apply/cancel, a retained
  identity-bound verified staged artifact, live out-of-band progress, typed
  errors, and the Tauri seam plus generated projections.
- **Out of scope:** the admission lease (`g02.042`), product behaviour, and any
  weakening of verification or the gate.
- **Repo constraints:** follow `AGENTS.md`; keep `apply` accepting only
  `VerifiedArtifact`.

## Important Context

- **Planning lineage:** contract 018 amended 2026-09-22; the release is
  `0.2.0`, operator-owned, after 041 and 042 merge.
- **Open tension:** the earlier design deliberately rejected a half-finished
  transfer type. The amendment reverses that stance on purpose; the staged
  handle must hold only verified bytes, so the reversal does not move the trust
  boundary.

### UI Design Brief

Not applicable. This is a Rust protocol and host seam.

## Suggested Next Move

Read the amended contract 018 section and the canonical task, then implement the
protocol types and the controller split. Keep the `VerifiedArtifact`-only
invariant visible in the types — the review will inspect it.

## Completion Protocol

Work on the queue-owned branch, not `main`. Implement the task, run the focused
`longhorn-update` selectors and `check:bindings`, then `effigy qa`, open a PR,
and report `ready_for_review` through the Queue callback helper. Fix review
findings on the same branch. Stop and report if the staged handle cannot hold
only verified bytes without changing the `UpdateInstaller` trait, or if any
design would weaken the gate.
