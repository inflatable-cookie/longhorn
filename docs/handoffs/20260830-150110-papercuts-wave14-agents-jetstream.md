---
title: Papercuts wave 14 AGENTS-audit selector and jetstream workspace worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom / papercuts orchestrator
created: 2026-08-30
updated: 2026-08-30
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260830-150110-papercuts-wave14-agents-jetstream.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
---

## What This Thread Was Doing

Wave 13 closed the dead-pid discovery sweep. Two leftovers remain.

`effigy check:agent-instructions` is not defined here, so the standard
AGENTS review command fails before the audit. The consumer-safe
installed-Northstar fallback exists, but it is not visible from this
repo's task surface.

`examples/command-system-proof/rust/jetstream/Cargo.toml` uses
`.workspace = true` without being a root workspace member and without
its own `[workspace]` table. Northstar's repository-scope Rust audit
dies there before it can inventory units.

You are the Longhorn implementation worker. Make the AGENTS audit
reachable, and make jetstream's workspace membership explicit or
isolate it. Leave the URL-primitive papercut alone.

## Why It Matters

The next AGENTS review and the next Rust audit both stop on routing,
not on the files they were asked to inspect.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `265b74f1867424c2dbd561279673dbce3d1b433c`
- **Pushed main verification:** local `HEAD` and `origin/main` both resolved
  to that SHA before this handoff was created.
- **Planning checkout:** clean before this handoff file was created.
- **Worker mode:** implementation worker dispatched by the orchestrator.
- **Worker branch:** `worker/papercuts-wave14-agents-jetstream`
- **Worker worktree:** launcher first. `.agents.local.env` is absent in
  the planning checkout; if the launcher did not supply a clean
  dedicated non-`main` worktree, ask the operator for
  `AGENTS_WORKTREE_CONTAINER_DIR` before creating a fallback. Never use
  `/tmp`.
- **Required sibling worktree links:** `none`
- **Ready work items, in order:**
  1. Consumer repo omits target-local AGENTS audit selector — add a
     target-local read-only `check:agent-instructions` alias, or
     document the installed-Northstar fallback on the Longhorn
     instruction surface so `AGENTS.md` names the exact command. Do not
     copy Northstar's Rhai into this repo. Do not invent a second audit.
     Existing `qa:docs:agent-defaults` is a different check (forbidden
     `--repo .`); keep it. Fallback shape when documenting:
     `effigy --repo <installed-northstar> northstar/check:agent-instructions <this-repo>`.
  2. Rust audit discovery rejects an excluded example workspace —
     `examples/command-system-proof/rust/jetstream/Cargo.toml` must
     either join the root workspace members or carry its own
     `[workspace]` table. The root comment already says non-member
     prototypes isolate themselves that way; isolation is the option
     that matches current layout, membership is fine if it is cleaner.
     The same-directory `loophole` crate uses the same
     `.workspace = true` / not-a-member pattern — if the audit still
     fails after jetstream, treat that sibling as the same membership
     bug. Do not hunt the whole `examples/` tree unless the audit still
     dies after those two.
- **Out of scope:** Endpoint URL duplication
  (`longhorn-update::EndpointUrl` /
  `longhorn-licence::ActivationUrl`); Figmatic; release gates; GitHub
  workflows.
- **Canonical refs:** `PAPERCUTS.md`; `AGENTS.md`; `effigy.toml`; root
  `Cargo.toml` (members + the prototypes-are-not-members comment);
  `examples/command-system-proof/rust/jetstream/Cargo.toml`; Northstar
  `references/modes/agent-instruction-review.md` consumer fallback.
- **Required validation:** `effigy check:agent-instructions` succeeds
  from this repo, or `AGENTS.md` names the installed-Northstar command
  and that command runs. `cargo metadata --manifest-path Cargo.toml
  --no-deps` no longer dies on jetstream. Do not require a full
  Northstar rust-quality audit run.
- **PR URL:** pending
- **Merge authorisation:** absent; do not merge

## Boundaries

- Selector/docs plus jetstream membership. Do not promote a shared URL
  primitive. Do not merge.

## Important Context

- Filed during the 2026-08-29 refresh
  (`docs/triage/20260829-154310-refresh-observations.md`). The triage
  note can stay open on handoff retention; close or point the
  agent-instruction bullet once this PR lands.
- **Report to:** the operator.

## Suggested Next Move

Read this file from the top. Run the worktree-safety preflight. After
the committed `HEAD` handoff checks out, skip sibling links (`none`),
then take the AGENTS selector and jetstream membership in that order.

## Completion Protocol

### Before you start

1. Read this handoff path. Its `worker_mode: implementation` and
   `dispatch_authority: orchestrator` metadata activate worker mode. Then
   run `git rev-parse --show-toplevel`, `git branch --show-current`,
   `git status --porcelain`, and `git worktree list --porcelain`.
2. If the current root is a registered worktree, status is empty, and the
   branch is not `main`, accept it. Record the actual path/branch.
3. If the launcher supplied a dirty or `main` worktree, stop and report
   it. `.agents.local.env` was absent; ask before creating a fallback.
   Never use `/tmp`.
4. From the selected worktree, record the repository-relative path
   `docs/handoffs/20260830-150110-papercuts-wave14-agents-jetstream.md`.
   Confirm `HEAD == origin/main`, ancestor
   `265b74f1867424c2dbd561279673dbce3d1b433c`, and that relative path in
   `HEAD`. Load
   `git show HEAD:docs/handoffs/20260830-150110-papercuts-wave14-agents-jetstream.md`.
   If the absolute dispatch file differs, stop. The `HEAD` copy is
   canonical.
5. Required sibling list is `none`. Skip link setup.
6. Read `AGENTS.md` and `PAPERCUTS.md`.

### When the assigned runway is complete

1. Update `PAPERCUTS.md`. Push a PR. Do not merge.

### Review and merge path

Awaiting orchestrator review. Merge is operator-authorised only.

- **Closeout refs:** `PAPERCUTS.md`; this handoff; the PR.

### Handoff closeout

Leave the URL-primitive papercut open.
