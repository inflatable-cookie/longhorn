# 204 Event Scoping Decision

Status: ready
Owner: Tom
Roadmap: g02.022 batch 1
Governing refs: contract 010; memo 023 (M-events)
Depends on: none
Auto-start next card: no

## Objective

Decide whether bridge events are per-window targeted or hint-only, record the
decision in contract 010, and make the code match.

## Why this exists

`TauriBridgeEventSink::emit` uses `app.emit`
(`crates/longhorn-tauri-bridge/src/events.rs:43`) and `publish_domain_event`
checks session + epoch but not read authority
(`handler/publication.rs:20-35`). Every webview in the app that listens
receives full typed payloads for all sessions and domains — including windows
whose negotiated authority is `ReadAuthority::None`. Client-side cursor
filtering (`packages/longhorn/src/bridge/runtime/connection.ts:170-187`) is
advisory. The rest of the surface is per-caller authorized; this is the one
asymmetric edge, and the audit could find no recorded decision for it.
Contract 010 calls events "projections and invalidation hints" while the
bridge publishes full payloads.

## Scope

- the decision, recorded in contract 010 with its reason
- `crates/longhorn-tauri-bridge` event sink and publication path
- `packages/longhorn` bridge runtime, if the decision changes what clients
  receive
- a cross-caller receipt negative test, whichever way the decision goes

## Steps

1. Decide. Option A: per-window `emit_to` keyed on the publishing session's
   caller — payloads stay rich, delivery matches authority. Option B: events
   become hint-only (domain + cursor, no payload) — multi-window
   differentiated-authority apps stay safe by construction, clients refetch.
   Weigh: A keeps today's client code but trusts every host to target
   correctly; B is simpler to reason about and costs a round trip.
2. Check the GPUI seam before committing to A — the event-sink trait has two
   hosts, and targeting must be implementable on both.
3. Implement the decision.
4. Add the negative test: a window without read authority does not receive
   the payload (A), or events provably carry no payload (B).
5. Amend contract 010's event language in the same change.

## Do Not

- Pick A because it is less code. The question is which property multi-window
  apps need; the code follows.
- Leave the advisory client filter standing as if it were enforcement,
  whichever way the decision goes.

## Acceptance Criteria

- [ ] the decision is recorded in contract 010 with its reason
- [ ] delivery and read authority agree, by mechanism or by construction
- [ ] the cross-caller negative test exists and fails before the fix if A

## Evidence Required

- the decision record
- the implementation and the negative test
- both hosts' event sinks conform
- `effigy qa` green

## Stop Conditions

Stop if per-window targeting turns out to be unimplementable on one host's
event model — that makes B forced, and the contract should say so.
