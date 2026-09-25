---
title: "g02.045 agent-answerable file selection"
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
status: ready-to-launch
base_required: pushed-main
roadmap: docs/roadmaps/g02/045-agent-answerable-file-selection.md
queue_dispatch: northstar-queue
queue_approval: "Operator Tom's 2026-09-24 cross-project direction asked the Longhorn Chatterbox to carry g02.045 through the approved Northstar planning and Queue delivery path, independent review, merge, and consumer handoff/release when appropriate."
queue:
  capability: complex
---

## What This Thread Was Doing

The Longhorn Chatterbox settled a common agent-control blocker: Tauri JS file
and folder pickers stop an MCP-driven app until a human chooses a path. The
operator chose a Longhorn-owned selection seam for `open`, directory, and
`save`, available in dev and packaged opt-ins. Contract 022 and g02.045 were
promoted at `4b12389a`. This handoff starts implementation through Queue.

## Why It Matters

Figmatic's Import project flow calls `@tauri-apps/plugin-dialog` directly
from a UI handler. Its command bridge is unsupported and its dev build omits
`evaluate`, so neither route can answer the picker. Figmatic g01.050 is
blocked before live import proof; linking Longhorn `v0.2.0` or current source
cannot supply the missing tools.

## Current State

- Integration repo: `/Users/tom/Dev/projects/longhorn`; `main` was clean and
  synchronized at `4b12389a` before this handoff. Verify the pushed handoff
  in the selected worktree's `HEAD` before any implementation change.
- g02.045 is the sole ready Longhorn card. No `answer_selection` or
  `reject_selection` implementation existed at dispatch preparation.
- Canonical authority: `docs/contracts/022-agent-app-control.md`,
  `docs/roadmaps/g02/045-agent-answerable-file-selection.md`,
  `docs/roadmaps/g02/README.md`, and `docs/roadmaps/generation-index.md`.
- Owned Longhorn paths and reserved closeout surfaces are the card's Dispatch
  manifest. No concurrent Longhorn sibling lane was ready at submission.
- Required sibling worktree links: none. Figmatic adoption is separately
  owned; its current worker/workspace/branch must not be altered here.
- Worker routing: Queue's automatic complex-capability pool. No by-request
  frontier profile was selected; frontier-worker justification: none.

## Boundaries

- Implement only the contract 022 and g02.045 Longhorn seam. The agent path
  answers a selection before an OS panel opens; the human path retains the
  original plugin behavior even while agent control is active.
- Include `save` target selection but no Longhorn file write. Preserve
  loopback, bearer, Origin, compile-time opt-in, and the default build's
  absence of agent control. `evaluate` remains separate and unnecessary.
- HTML `<input type="file">`, Rust-side pickers, native OS panel control,
  remote listeners, global plugin monkey-patching, and Figmatic repository
  writes are out of scope. Consumer workflow and path policy stay with the app.
- Follow `/Users/tom/Dev/projects/longhorn/AGENTS.md`. Do not edit
  `.github/workflows/` without explicit human approval. Stage explicit paths.

## Important Context

The route uses the existing MCP server and resource subscription path, not a
second server. Pending requests need an id, bounded lifetime, readable state
for late subscribers, typed expiry and malformed-answer errors, and explicit
rejection that yields the plugin-compatible cancellation result. An active
server alone cannot redirect a human picker. The card's oracle covers the
actual agent-versus-human routing, feature states, save authority, and a live
Figmatic fresh-leaf import.

Figmatic Chatterbox owns the separate consumer adaptation and has retained
g01.050. It is arranging a one-time operator native import to unblock that
worker; that does not waive Figmatic's app-driving policy or close g02.045.
Coordinate the Longhorn source/API and an acceptance candidate through the
Queue report so Chatterbox can give Figmatic a precise handoff. Do not write
Figmatic from this Longhorn task. If its live acceptance cannot run before the
Longhorn PR is ready, report the missing evidence and keep the claim open.

### UI Design Brief

Not applicable. Existing consumer picker controls do not change.

## Suggested Next Move

Read contract 022 and the full g02.045 card, then trace the core MCP resource
and tool dispatch, Tauri host bridge, and Longhorn TypeScript agent-control
entry points. Implement the bounded selection route with focused fixtures
before asking Figmatic for source-linked acceptance. Raise a decision blocker
if distinguishing agent-originated from human-originated calls would change
human picker behavior or require a new product choice.

## Completion Protocol

Use the card's acceptance and review oracle, focused agent-control checks,
feature-state/absence proof, then `effigy qa` after the coherent batch. Name
any live Figmatic evidence separately from fixtures. Open one PR and report
its exact head, tests, the consumer API, and remaining evidence through the
Queue run. Queue owns independent exact-head review, revisions, merge, and
canonical closeout; do not self-review or self-merge. A release/tag/npm publish
requires a separate operator release decision after the merged artifact and
consumer need are concrete.

After Longhorn merge, Chatterbox coordinates a Figmatic-owned call-site
adoption and live fresh-leaf folder-selection proof. Close g02.045 only when
the card's acceptance evidence is honest and linked; otherwise keep the
specific consumer gate visible. The next Longhorn task is not yet selected.
