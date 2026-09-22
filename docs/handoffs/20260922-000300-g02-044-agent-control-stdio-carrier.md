---
kind: northstar-handoff
title: "g02.044 — Agent-control stdio carrier"
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
queue_approval: "Operator direction 2026-09-22 from the Swallowtail requirement: a Longhorn-owned stdio carrier fronting the single contract 022 server, included in Longhorn 0.2.0."
roadmap: docs/roadmaps/g02/044-agent-control-stdio-carrier.md
tags: [coordination, handoff, worker, pr, agent-control]
---

## What This Thread Was Doing

Swallowtail raised a consumer requirement: some harnesses (Claude Code, the
Claude Agent SDK sidecar, Grok Build ACP, Codex) may not accept a
consumer-supplied streamable-HTTP MCP server entry, so Bovine Desktop needs a
**stdio carrier for the same contract 022 server instance** — owned by Longhorn
or the app, never by Swallowtail. Longhorn accepted it and promoted the existing
Tier A "agent-control stdio proxy client" candidate into this lane. The operator
directed it into `0.2.0`.

## Why It Matters

Contract 022's server is streamable HTTP on loopback. Figmatic reaches it
directly; Bovine Desktop drives harnesses that may only speak stdio. The carrier
is what lets those harnesses reach the one server without anyone standing up a
second production MCP server — which is exactly what the 2026-09-22 direction
withdrew.

## Current State

- **Done:** the requirement is accepted, the task is promoted, and the operator
  put it in the `0.2.0` scope.
- **Still open:** the binary, discovery/bearer path, proxy, semantics position,
  and tests.
- **Active spec lane:** none.
- **Current task:** `docs/roadmaps/g02/044-agent-control-stdio-carrier.md` (`g02.044`).
- **Canonical refs:** contract 022 (`Availability And Security`, `Tool Surface`,
  `Discovery`).
- **Remaining continuation envelope:** none.
- **Lane budget / pause signal:** normal; stop on the task's stop conditions.
- **Required sibling worktree links:** none.
- **Key files:**
  - `crates/longhorn-agent-control/src/discovery.rs`
  - `crates/longhorn-agent-control/src/` (the server and tools)
  - a new crate/bin for `longhorn-agent-control-client`

## Boundaries

- **In scope:** the opt-in stdio carrier, discovery, bearer/loopback pass-through,
  the stdio proxy, and the listen/resources position.
- **Out of scope:** any second catalogue, registry, lease, or listener; any
  Swallowtail-owned production bridge; the harness-side MCP client configuration.
- **Repo constraints:** follow `AGENTS.md`. The carrier must hold no catalogue of
  its own and expose no new listener.

## Important Context

- **Planning lineage:** contract 022 amended and contract 023's production role
  withdrawn on 2026-09-22; `0.2.0` ships four lanes (`g02.041`-`g02.044`).
- **Decisions:** the carrier fronts the discovered instance; listen and resources
  over stdio are deliberately not claimed in the first pass unless a route needs
  them; distribution (crate bin vs npm) is decided inside the task.
- **Open tension:** Swallowtail is compiling per-route evidence in parallel. It
  does **not** gate dispatch — validate against at least one stdio harness
  end-to-end, and against the routes that evidence names as it lands.

### UI Design Brief

Not applicable. This is a Rust binary and a transport.

## Suggested Next Move

Read the canonical task and `longhorn-agent-control`'s discovery module, then
stand up the binary and the stdio↔HTTP proxy. Keep the semantics parity visible
in the tests — the review will inspect the carrier-to-server path.

## Completion Protocol

Work on the queue-owned branch, not `main`. Implement the task, run the focused
agent-control selectors plus an end-to-end stdio run, then `effigy qa`, open a
PR, and report `ready_for_review` through the Queue callback helper. Fix review
findings on the same branch. Stop and report if the carrier would need its own
catalogue, registry, lease, or listener, or if a route can only be served by a
Swallowtail-owned production listener.
