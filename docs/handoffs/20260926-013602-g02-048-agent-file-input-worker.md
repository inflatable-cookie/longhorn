---
title: Longhorn g02.048 agent file input worker handoff
kind: northstar-handoff
handoff_mode: worker-pr-loop
worker_mode: implementation
dispatch_authority: orchestrator
handoff: single-file-path-only
status: ready-to-launch
owner: Tom
created: 2026-09-26
updated: 2026-09-26
handoff_path: /Users/tom/Dev/projects/longhorn/docs/handoffs/20260926-013602-g02-048-agent-file-input-worker.md
base_required: pushed-main
queue_dispatch: northstar-queue
queue_approval: "Tom chose the picker follow-up lane on 2026-09-26 ('Let's do 1 and 2 now'), confirmed inline agent-supplied bytes with no Longhorn path reads and Soundcheck as consumer-owned acceptance, and directed Chatterbox to dispatch through Northstar Queue."
queue:
  dependsOn: [258d3724-5d2d-4682-97d7-3a4f08e22ec5]
tags: [coordination, handoff, worker, agent-control]
---

## What This Thread Was Doing

Agent-control can answer JS and (after g02.047) Rust picker calls, but not an
HTML `<input type="file">`. This worker adds the `set_file_input` tool under
[g02.048](../roadmaps/g02/048-agent-file-input.md). Read the card and contract
022's `Tool Surface` and `Agent-Answerable Selection` sections (amended
2026-09-26) before editing.

## Why It Matters

Soundcheck's manifest comparison reads a file from an HTML input. An agent's
untrusted `click` opens no panel and sets no files, so today the agent must use
the paste-JSON workaround or OS-level control.

## Current State

- This task depends on g02.047 (Queue `258d3724-5d2d-4682-97d7-3a4f08e22ec5`),
  which owns the same shim, catalogue, and skill paths. Start from `main`
  after it merges.
- The shim already builds a `DataTransfer` for `drag`
  (`packages/longhorn/src/agent-control/shim.ts`) and marks agent origin for
  input tools.
- g02.049 (release tooling) runs independently.

## Boundaries

- Own the paths in the card's dispatch manifest. Work only in the
  Queue-provided non-main worktree.
- The agent is the only byte source: no path parameter and no filesystem read.
  Keep the 8 MiB decoded cap. Do not open any OS panel.
- No `.github/workflows/`, Effigy, or consumer-repo edits. No tag, publish,
  or workflow dispatch. Do not edit the card's lifecycle block or closeout
  surfaces.
- UI design brief: not applicable.

## Important Context

- The tool must work in a packaged `agent-control` build without
  `agent-control-evaluate`, and follow the child-webview targeting rules.
- `effigy check:agent-control-skill` locks the skill tool table to the
  catalogue.
- If WKWebView refuses `files` assignment from a constructed `DataTransfer`,
  stop with evidence (a card stop condition).

## Suggested Next Move

Read the card and contract. Prove early in the real webview that assigning
`files` from a constructed `DataTransfer` works and `file.text()` returns the
content. Then add the catalogue entry with validation, the shim handler, host
wiring, fixtures for every oracle row, and docs. Run the focused checks, the
skill and release-absence checks, and `effigy qa`. Open a PR for Queue's
independent reviewer.

## Completion Protocol

Queue owns review, revision, and merge. Record changed paths, commands, fixture
evidence, and residual limits in the PR, with the Soundcheck row marked pending
consumer adoption. Queue closes out g02.048 after merge.

Disposition trigger: transient worker handoff; prune through the normal
Northstar lifecycle after Queue closeout.
