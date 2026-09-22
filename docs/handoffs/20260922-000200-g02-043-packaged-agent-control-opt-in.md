---
kind: northstar-handoff
title: "g02.043 — Packaged agent-control opt-in"
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
queue_approval: "Operator direction from the Figmatic lane, 2026-09-22: a packaged Figmatic build must include the contract 022 server behind a compile-time opt-in, with `evaluate` split off; this lands in Longhorn 0.2.0."
roadmap: docs/roadmaps/g02/043-packaged-agent-control-opt-in.md
tags: [coordination, handoff, worker, pr, agent-control]
---

## What This Thread Was Doing

A Figmatic chatterbox request under operator direction: a packaged build must be
able to ship the contract 022 agent-control server so an external harness drives
the running app with the operator's agency, through a consumer-registered typed
command catalogue. Today the surface is behind an off-by-default `dev` feature
and contract 022 says a release build contains none of it. Contract 022 was
amended 2026-09-22 and contract 023's competing production MCP role withdrawn.
This lane implements the opt-in.

## Why It Matters

The server is the original production intent. Figmatic is dropping in-app
Swallowtail, AI settings, and chat — the harness is external. The compile-time
opt-in is what lets a consumer ship it without making it present in every store
build, and the `evaluate` split keeps arbitrary JS execution out of the packaged
surface.

## Current State

- **Done:** contracts amended; this task and handoff are committed; the operator
  approved the change and the `0.2.0` scope.
- **Still open:** the feature rename, the `evaluate` split, the re-scoped absence
  proof, and the example/skill updates.
- **Active spec lane:** none.
- **Current task:** `docs/roadmaps/g02/043-packaged-agent-control-opt-in.md` (`g02.043`).
- **Canonical refs:** contract 022 (`Availability And Security`, `Tool Surface`);
  contract 023 (withdrawn for production MCP).
- **Remaining continuation envelope:** none — the Figmatic tool catalogue is
  Figmatic's own follow-on lane after this API exists.
- **Lane budget / pause signal:** normal; stop on the task's stop conditions.
- **Required sibling worktree links:** none.
- **Key files:**
  - `crates/longhorn-tauri-agent-control/Cargo.toml`
  - `crates/longhorn-tauri-agent-control/src/lib.rs`
  - `crates/longhorn-agent-control/src/lib.rs`
  - `scripts/verify-agent-control-release-absence.ts`

## Boundaries

- **In scope:** the `agent-control` feature (renamed from `dev`), the
  `agent-control-evaluate` split, the re-scoped absence proof, and the
  example/skill/doc updates.
- **Out of scope:** a second listener, any Swallowtail routing, the Figmatic
  typed command catalogue, and any weakening of loopback/token/Origin.
- **Repo constraints:** follow `AGENTS.md`; the default build must stay free of
  the surface, and the server must never start without an explicit application
  call.

## Important Context

- **Planning lineage:** contract 022 amended 2026-09-22; `0.2.0` ships three
  lanes (`g02.041`, `g02.042`, `g02.043`).
- **Decisions:** `evaluate` is omitted from the packaged posture (the safer
  cut); the consumer's registered catalogue is the allowed agency.
- **Open tension:** the absence proof currently asserts a *release* build has no
  surface. After this change the invariant is "no `agent-control` feature → no
  surface"; the proof must express all three feature states.

### UI Design Brief

Not applicable. This is Rust feature gating and a host seam.

## Suggested Next Move

Read the amended contract 022 sections and the canonical task, then rename the
feature and split `evaluate`. Keep the absence proof meaningful: assert the
three states rather than only on/off.

## Completion Protocol

Work on the queue-owned branch, not `main`. Implement the task, run the focused
agent-control selectors and `check:agent-control-release-absence`, then
`effigy qa`, open a PR, and report `ready_for_review` through the Queue callback
helper. Fix review findings on the same branch. Stop and report if the default
build cannot stay free of the surface, or if the server could start without an
explicit application call.
