# 233 Agent Control Semantic Tools

Status: done 2026-08-19
Owner: Longhorn maintainers
Roadmap: g02.032
Governing refs: contract 022; contracts 006, 010; memo 024
Depends on: Card 232
Auto-start next card: yes — Card 234 in the same lane

## Objective

The plugin's remaining `Unsupported` answers become real: `snapshot`,
`click`, `type`, `press`, `scroll`, `drag`, and `wait_for` run through the
Card 232 shim, and agents can subscribe to console output, page errors,
and navigation events.

## Scope

- **Shim injection.** The plugin injects the Card 232 shim in dev
  (initialization script per webview), idempotently — a reloaded page
  re-arms itself. Injection exists only behind the `dev` feature; the
  release-absence scan must stay clean with the shim asset gated too.
- **Tool wiring.** Each semantic tool marshals through the capture
  bridge's evaluate path into typed shim calls, mapping shim outcomes
  onto the core vocabulary (`UnresolvedRef`, `WaitTimeout`,
  `EvaluationFailed`). `wait_for` pacing and timeout live host-side; the
  shim only answers the predicate.
- **Event push.** Implement contract 022's observability channel:
  `subscriptions/listen` per the MCP revision in use, carrying console
  messages, page errors, and navigation events captured by the shim and
  forwarded through the plugin. Buffering is bounded and drop-oldest with
  a drop counter surfaced to the subscriber — no unbounded queue, no
  silent loss.
- **Conformance.** Extend the plugin's mount fixtures: every tool answers
  over a real loopback; a snapshot → click → wait_for round-trip against
  a real page; ref-staleness surfaced as `UnresolvedRef` end to end; a
  listen stream receives a console line and a navigation event; two
  clients' subscriptions do not cross.

## Acceptance Criteria

- [x] no tool in the contract 022 surface answers `Unsupported` on the
      Tauri host with a webview present (native-surface provider absence
      unchanged)
- [x] the round-trip fixture passes over real loopback
- [x] listen streams deliver console, error, and navigation events with
      bounded buffering and an honest drop counter
- [x] release-absence scan stays green feature-off with the shim asset
      and injection code gated
- [x] `effigy qa` passes

## Validation

`effigy qa`; the release-absence scan both directions; plugin fixture
suite.

## Stop Conditions

- rmcp's server surface cannot carry `subscriptions/listen` on the
  revision current clients negotiate — stop and report with what it does
  support rather than inventing a side channel;
- tool marshalling needs the core vocabulary or plugin public surface to
  change shape — report first, per the standing rule.

## Closeout

Status: done 2026-08-19, same branch and worktree as Card 232.

The plugin injects the Card 232 IIFE as a Tauri initialization script
(plus `eval` on already-created windows). Semantic tools marshal through
`evaluate` + `JSON.stringify`; shim `{ok:false}` maps onto the core
`ToolError` vocabulary. `wait_for` polls host-side at 50 ms.

rmcp 3.1.3 listen works on 2026-07-28. The sink rejects custom
notifications, so page events are three resources
(`longhorn://agent-control/{console,error,navigation}`) and
`resources/updated` on the listen stream. Contract 022 records the
mapping. Drop-oldest lives in the shim ring; the drop counter is on the
resource body.

Mock-runtime mount fixtures cannot execute JS: snapshot answers
`evaluationFailed`, not the old g02.032 `unsupported`. Resources/list
and listen acknowledgment are proved over real loopback. The packaged
round-trip is Card 234.

## Continuation

Card 234 closes the milestone with the packaged end-to-end proof.
