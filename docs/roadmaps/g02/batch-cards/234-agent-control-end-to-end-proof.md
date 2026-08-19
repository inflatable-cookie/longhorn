# 234 Agent Control End-to-end Proof

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.032
Governing refs: contract 022 (Required Evidence is this card's checklist);
contracts 006, 020; memo 024
Depends on: Card 233
Auto-start next card: no — the feature closes here

## Objective

The whole point, proved: an agent drives a packaged consumer-shaped app
end to end — semantic snapshot, clicks, typed text, waits, screenshots,
commands — while the app never holds OS focus and the operator's pointer
never moves. Contract 022's Required Evidence section closes or is
honestly amended.

## Scope

- **Proof app.** Extend `examples/agent-control-proof` into a
  consumer-shaped surface: a small form-and-list UI (enough for click,
  type, drag-reorder, and a navigation), contract-006 commands behind a
  native-menu-shaped action, and the ticking-hue freshness page retained.
- **End-to-end run.** A committed driver (per the Card 231 pattern)
  builds the packaged `.app`, launches it unfocused, and executes the
  full loop through a real MCP client connection: snapshot → click(ref) →
  type → wait_for → screenshot → command, asserting outcomes from the
  DOM. Record that the app never held focus (System Events check, as the
  spike did) and that no OS pointer motion exists in the path.
- **Two-client interleave.** Two concurrent MCP clients drive one
  instance — interleaved snapshots and inputs, independent listen
  streams — with no cross-talk and no ref invalidation across clients.
- **Contract closeout.** Walk contract 022's Required Evidence list item
  by item: satisfied items cite their proof (this card, PR 3, PR 4);
  anything narrowed (per-host evidence per contract 020, native-surface
  provider absence, another-Space) is amended in the contract explicitly
  rather than left implied. Untrusted-event limits documented where
  consumers read (crate/package README or composition guide section).
- **Milestone and lane closeout.** g02.032 and the runway lane close;
  the g02 README's agent-control block and the generation-index pointer
  hand back to the orchestrator.

## Acceptance Criteria

- [ ] the packaged proof run passes with the app unfocused throughout,
      evidence committed (driver output, screenshots, focus check)
- [ ] two-client interleave passes on one packaged instance
- [ ] every contract 022 Required Evidence item cites proof or carries an
      explicit amendment in the contract (amendment = report to the
      orchestrator first, then the edit lands with the evidence)
- [ ] untrusted-event limits and the DOM-relative waiting rule are
      documented for consumers
- [ ] release-absence scan green feature-off on the final tree
- [ ] `effigy qa` passes

## Validation

`effigy qa`; the packaged end-to-end driver on a macOS host; the
release-absence scan; `effigy doctor`.

## Stop Conditions

- the end-to-end loop exposes a semantic-surface behavior that
  contradicts contract 022 (ref instability under interleave, event loss
  without a counter, focus stolen by any path) — stop, record, report;
- contract amendment wants to widen scope (native surfaces, trusted
  input, remote access) rather than narrow honestly — that is a new
  contract conversation, not a closeout edit.

## Continuation

The agent-control runway ends here. Consumer adoption (the five sibling
apps composing the plugin) is per-app work outside Longhorn's runway;
the orchestrator names the next planning checkpoint at merge.
