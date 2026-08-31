# Papercuts wave 27 — Longhorn AlreadyAtTarget wire code

handoff: single-file-path-only
status: pr-open
owner: Tom / papercuts orchestrator
created: 2026-08-31
updated: 2026-08-31
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260831-203639-papercuts-wave27-already-at-target.md
base_required: pushed-main
tags: [coordination, handoff, worker, pr, papercuts]
pr: https://github.com/inflatable-cookie/longhorn/pull/20
head: fb4cebed7fbb45b071556e429a083c1f4ca3e6d8

## What This Thread Is Doing

Longhorn has a typed Rust `ForkNavigationError::AlreadyAtTarget`, but the
wire rejection projection collapses it into `invalidRequest` plus detail text.
Poodle HistoryCenter needs to distinguish `AlreadyAtTarget` from
`UnknownEntry`; downstream code currently has to sniff the detail string.

You are the Longhorn implementation worker. Add the smallest explicit wire
rejection code for the existing `AlreadyAtTarget` error and prove the mapping.
Use the existing naming, serialization, and compatibility conventions in the
history-tree protocol. Keep all other rejection mappings unchanged. Close the
matching Longhorn entry in `PAPERCUTS.md` and record focused evidence.

## Why It Matters

An operator notice must not depend on host-language copy. A stable wire code
lets Poodle/Loophole map the existing error without parsing diagnostic detail.

## Current State

- **Repository:** `/Users/tom/Dev/projects/longhorn`
- **Planning branch:** `main`
- **Planning base commit:** `956e19716463c3ba2734b3bb85ec7bf982b8cc95`
- **Worker branch:** `worker/papercuts-wave27-already-at-target`
- **Worker head:** `fb4cebed7fbb45b071556e429a083c1f4ca3e6d8`
- **PR:** https://github.com/inflatable-cookie/longhorn/pull/20
- **Downstream owners:** Poodle/Loophole consume the wire code later; do not
  edit either repository in this lane.
- **Related source entry:** Longhorn `PAPERCUTS.md`, “Longhorn wire rejection
  lacks AlreadyAtTarget” — closed on this branch.

## Worker Result

- Added `ForkNavigationRejectionCode::AlreadyAtTarget` (`alreadyAtTarget`).
- Focused protocol tests prove dedicated code + existing `unknownTarget` mapping.
- Regenerated history-tree bindings; closed papercut.
- Validation: history-tree `protocol` binary green; bindings check current;
  `effigy qa:docs` and `effigy qa:northstar` green; `git diff --check` clean.
- Downstream note: Loophole still maps `already_at_target` to `invalidRequest`
  until a separate adoption lane switches to `alreadyAtTarget`.

## Boundaries

- Longhorn only: protocol/error mapping, focused tests, docs/evidence, and the
  matching Longhorn papercut closeout.
- Do not edit Poodle, Loophole, Pulse, or downstream pins.
- Do not change the Rust navigation semantics or introduce a new navigation
  behavior; this is a projection/code classification repair.
- Do not make downstream consumers parse a new detail string. Preserve the
  existing detail for compatibility if the current protocol exposes it.
- Do not fold in empty-branch checkout, CS20 group identity, or unrelated
  history API work.

## Required Validation

- Focused Longhorn history-tree/protocol tests covering `AlreadyAtTarget` and
  at least one existing rejection mapping.
- Relevant `effigy` test/docs selectors for the changed surface.
- `effigy qa:docs` and `effigy qa:northstar`.
- `git diff --check`.

## Before You Start

1. Read this handoff, `AGENTS.md`, and `PAPERCUTS.md`.
2. Confirm the checkout is a worker worktree, not `main`, and that its
   `HEAD` contains this handoff and starts from the planning base above.
3. Inspect the current Rust error-to-wire mapping and its serialized schema
   before choosing the exact code spelling.

## Completion Protocol

1. Keep the implementation and proof bounded to this entry.
2. Commit and push the worker branch.
3. Open a PR against `main`; do not merge from the worker lane.
4. Report the exact head, PR URL, changed files, focused evidence, and any
   downstream adoption note to the papercuts orchestrator.

## Review and Merge Path

The papercuts orchestrator reviews the exact head and merges after the checks
settle. Downstream Poodle/Loophole adoption remains a separate lane.
