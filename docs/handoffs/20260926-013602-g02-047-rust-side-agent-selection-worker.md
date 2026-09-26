---
title: Longhorn g02.047 Rust-side agent selection worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-09-26
updated: 2026-09-26
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260926-013602-g02-047-rust-side-agent-selection-worker.md
base_required: pushed-main
queue_dispatch: northstar-queue
queue_approval: "Tom chose the picker follow-up and release tooling lanes on 2026-09-26 ('Let's do 1 and 2 now'), confirmed explicit origin token, inline bytes, Longhorn-only release tooling, and Soundcheck as consumer-owned acceptance, and directed Chatterbox to dispatch through Northstar Queue."
tags: [coordination, handoff, worker, agent-control]
---

## What This Thread Was Doing

Longhorn `0.2.1` shipped agent-answerable JS `open`/`save` (g02.045). This
worker extends that seam to Rust-side picker calls under
[g02.047](../roadmaps/g02/047-rust-side-agent-selection.md). Read the card and
contract 022's `Agent-Answerable Selection` section (amended 2026-09-26)
before editing.

## Why It Matters

Soundcheck opens its backup-export and manifest-save pickers from Rust
(`blocking_save_file`). Those calls never cross the JS entry point, so an
agent cannot answer them today and has to fall back to OS-level control.

## Current State

- `main` carries the 2026-09-26 planning commit with the contract 022
  amendment and this card.
- `SelectionRegistry` (`crates/longhorn-agent-control/src/selection.rs`) and
  `longhorn_agent_control_begin_selection`
  (`crates/longhorn-tauri-agent-control/src/commands.rs`) implement the JS
  route. Origin lives in the page shim (`installOriginTracking` in
  `packages/longhorn/src/agent-control/shim.ts`).
- g02.049 (release tooling) runs in parallel on `scripts/` and `effigy.toml`.
  g02.048 (file input) waits for this task to merge.

## Boundaries

- Own the paths in the card's dispatch manifest. Work only in the
  Queue-provided non-main worktree.
- Origin is consumer-carried. Do not infer origin on the host, change the JS
  route's behavior, add a native fallback on the agent path, or let Longhorn
  write a selected path.
- No `.github/workflows/`, Effigy, or consumer-repo edits. No tag, publish,
  or workflow dispatch. Do not edit the card's lifecycle block or closeout
  surfaces.
- UI design brief: not applicable.

## Important Context

- The JS route's result shapes and lifecycle are the reference: single path or
  `null`, path array or `null`, save one path or `null`; reject yields `null`;
  expiry, shutdown, duplicate, and wrong-instance answers fail typed.
- A missing or malformed origin must behave as human.
- The release-absence scan must cover the new symbols both ways.

## Suggested Next Move

Read the card, contract 022, the JS selection route, and its tests. Add the TS
origin export, then the Rust entry point with fixtures for each oracle row,
then docs and the skill. Run the focused Rust/TS checks,
`effigy check:agent-control-release-absence`, and `effigy qa`. Open a PR for
Queue's independent reviewer.

## Completion Protocol

Queue owns review, revision, and merge. Record changed paths, commands,
fixture evidence, and residual limits in the PR, with the Soundcheck row
marked pending consumer adoption. Queue closes out g02.047 after merge.

Disposition trigger: transient worker handoff; prune through the normal
Northstar lifecycle after Queue closeout.
