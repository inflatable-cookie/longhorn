# Agent Control Runway Closeout

Date: 2026-08-19
Scope: g02.029-032 (Cards 227-234, PRs 2-5); memo 024; contract 022

## What shipped, in one day

The problem: agents testing consumer apps through OS computer use steal
focus and the pointer, making the operator's machine unusable. The
answer, now merged: a dev-build-only, stateless MCP streamable-HTTP
server inside each app. An agent discovers a running instance by file,
authenticates with a per-instance token, takes semantic snapshots with
live-DOM refs, dispatches untrusted in-page input, waits on DOM-relative
predicates, captures fresh screenshots of unfocused / occluded /
minimized windows, invokes contract-006 commands, and subscribes to
console/error/navigation events — while the app never holds OS focus and
the pointer never moves.

## Shape

- memo 024 → contract 022 (drafted, promoted, evidence-closed same day)
- g02.029 spike (PR 2): both runtime unknowns proved on the wire
- g02.030 core (PR 3): `longhorn-agent-control` — vocabulary, discovery,
  token, stateless rmcp server, guard-before-dispatch
- g02.031 host (PR 4): `longhorn-tauri-agent-control` — dev-gated mount,
  window scope, capture, release-absence scan in `qa`
- g02.032 surface (PR 5): TS shim (drift-locked bundle), semantic tools,
  listen-as-resources, packaged unfocused end-to-end proof

## Orchestration record

Four worker lanes dispatched by handoff, each reviewed independently
(re-run suites, re-run scans, evidence read directly) and merged on
operator authorisation. Every lane held scope; every deviation was
self-flagged and judged in review. Standing findings for future runways:

- the report-first rule for public-surface edits never produced a
  mid-run report in three consecutive lanes — correct fixes arrived
  self-flagged in PRs instead; next runway's handoffs should either
  scope expected seam-touches in, or accept flagged-in-PR as the norm;
- `.agents.local.env` remains unconfigured (PAPERCUTS entry, 2026-08-19);
- rrmcp facts worth keeping: 3.1.3 defaults to revision 2025-11-25 while
  supporting 2026-07-28; its listen sink rejects custom notifications
  (events ride as `resources/updated`); Claude Code negotiates
  2026-07-28 session-free.

## What's next

Consumer adoption: the five sibling apps compose the plugin behind `dev`
and hand their agents the discovery directory. Per-app work, outside
this runway.
