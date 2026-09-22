---
kind: northstar-handoff
title: "g02.042 — Exclusive admission lease"
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
roadmap: docs/roadmaps/g02/042-exclusive-admission-lease.md
queue:
  dependsOn: [2fa791d7-7484-4361-aec9-434ede971c14]
tags: [coordination, handoff, worker, pr, update-protocol]
---

## What This Thread Was Doing

The acowtancy Desktop lane (`g05.179`, contract 038) needs an exclusive mutation
admission barrier held through replacement. Today `UpdateGate::authorize`
returns `Approved | Deferred` from a point-in-time quiescence receipt, so
nothing stops new conflicting work starting while the bundle is replaced.
Contract 018 was amended on 2026-09-22 to require a held lease. This lane adds
it.

## Why It Matters

Replacing a running application while a write, index, or transfer starts is the
failure the barrier exists to prevent. The lease closes the window without the
gate learning what an application's operations are.

## Current State

- **Done:** contract 018 amended; this task and handoff are committed; the
  operator approved the amendment and the lease shape (a host-supplied
  admission-authority trait).
- **Still open:** the trait, the gate change, the seam, and the tests.
- **Active spec lane:** none.
- **Current task:** `docs/roadmaps/g02/042-exclusive-admission-lease.md` (`g02.042`).
- **Canonical refs:** contract 018 (`Exclusive Admission Lease`).
- **Remaining continuation envelope:** none.
- **Lane budget / pause signal:** normal; stop on the task's stop conditions.
- **Required sibling worktree links:** none.
- **Depends on:** `g02.041` (staged protocol) — this lane spans the interval it
  defines.
- **Key files:**
  - `crates/longhorn-update/src/gate.rs`
  - `crates/longhorn-update/src/restart.rs`
  - `crates/longhorn-tauri-update/src/handler.rs`

## Boundaries

- **In scope:** a host-supplied admission-authority trait and a held exclusive
  lease in `UpdateGate`, released only after `apply` returns.
- **Out of scope:** the staged protocol (`g02.041`), product behaviour, and any
  weakening of the gate.
- **Repo constraints:** follow `AGENTS.md`; Longhorn must learn no application
  operation.

## Important Context

- **Planning lineage:** contract 018 amended 2026-09-22; the release is `0.2.0`,
  operator-owned, after 041 and 042 merge.
- **Open tension:** quiescence is a point-in-time answer and the lease is a
  held one. Both stay: the receipt is the precondition, the lease is the
  barrier.

### UI Design Brief

Not applicable. This is a Rust gate and host seam.

## Suggested Next Move

Read the amended contract 018 section and the canonical task, then define the
admission-authority trait and thread the lease through authorization and
`apply`. The review will inspect the lease's lifetime.

## Completion Protocol

Work on the queue-owned branch, not `main`. Implement the task, run the focused
`longhorn-update` selectors and `check:bindings`, then `effigy qa`, open a PR,
and report `ready_for_review` through the Queue callback helper. Stop and report
if holding the lease requires the gate to know an application's operations, or
if any design would let an update install without an acquired lease.
