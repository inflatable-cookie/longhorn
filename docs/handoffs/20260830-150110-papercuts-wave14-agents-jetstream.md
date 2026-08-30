---
title: Papercuts wave 14 AGENTS-audit selector and jetstream workspace worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: completed-awaiting-review
owner: Tom / papercuts orchestrator
created: 2026-08-30
updated: 2026-08-30
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260830-150110-papercuts-wave14-agents-jetstream.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
---

## What This Thread Was Doing

Wave 13 closed the dead-pid discovery sweep. Two leftovers remained: the
missing AGENTS audit selector surface, and jetstream (plus loophole)
workspace membership that stopped Northstar Rust audit discovery.

## Why It Matters

The next AGENTS review and the next Rust audit both stopped on routing,
not on the files they were asked to inspect.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `265b74f1867424c2dbd561279673dbce3d1b433c`
- **Worker branch:** `t3code/papercuts-wave14-jetstream`
- **Worker worktree:** `/Users/tom/.t3/worktrees/longhorn/t3code-a1b8e9a2`
- **Done:**
  1. `AGENTS.md` Validation names the installed-Northstar command
     `effigy --repo <installed-northstar> northstar/check:agent-instructions <this-repo>`.
     No target-local Rhai copy; `qa:docs:agent-defaults` kept separate.
  2. Jetstream and loophole joined root `workspace.members` with private
     proof-member posture (`publish = false`, workspace package fields,
     `[lints] workspace = true`, crate-level docs). Isolation with
     `[workspace]` would break the command-system proof's private
     workspace copy.
  3. Both papercuts closed in `PAPERCUTS.md`. Triage agent-instruction
     bullet pointed closed.
- **Out of scope left open:** Endpoint URL duplication
  (`longhorn-update::EndpointUrl` /
  `longhorn-licence::ActivationUrl`); Figmatic; release gates; GitHub
  workflows.
- **Validation evidence:**
  - `cargo metadata` on root, jetstream, and loophole manifests succeeds;
    both packages report `publish = []` (private).
  - `cargo check -p longhorn-jetstream-command-artifact-proof -p
    longhorn-loophole-command-artifact-proof --offline` passes.
  - Installed-Northstar `northstar/check:agent-instructions` runs against
    this repo.
  - `effigy qa:docs:agent-defaults` passes.
- **PR URL:** https://github.com/inflatable-cookie/longhorn/pull/16
- **Merge authorisation:** absent; do not merge

## Boundaries

- Selector/docs plus jetstream membership. Do not promote a shared URL
  primitive. Do not merge.

## Important Context

- Filed during the 2026-08-29 refresh
  (`docs/triage/20260829-154310-refresh-observations.md`).
- **Report to:** the operator.

## Suggested Next Move

Orchestrator review of PR 16. Merge is operator-authorised only.

## Completion Protocol

### Review and merge path

Awaiting orchestrator review. Merge is operator-authorised only.

- **Closeout refs:** `PAPERCUTS.md`; this handoff; the PR.

### Handoff closeout

Leave the URL-primitive papercut open.
