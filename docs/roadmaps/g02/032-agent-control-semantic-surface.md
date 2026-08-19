# g02.032 Agent Control Semantic Surface

Status: done 2026-08-19
Owner: Longhorn maintainers
Created: 2026-08-19
Governing refs: contract 022; contracts 006, 010, 013
Depends on: g02.031 complete — true 2026-08-19 (PR 4)

## Outcome

An agent drives a packaged consumer-shaped app end to end while the app
never holds OS focus: semantic snapshot with live-DOM refs, ref-addressed
synthetic input, evaluate, wait_for, and contract-006 command invocation,
exposed as MCP tools.

## Generation Runway

- [x] [Card 232](batch-cards/232-agent-control-webview-shim.md) — TS shim
      in `longhorn`: semantic tree with stamped refs, synthetic dispatch
      (click, type, press, scroll, drag), wait_for predicate evaluation;
      ref-staleness fixtures.
- [x] [Card 233](batch-cards/233-agent-control-semantic-tools.md) — tools
      wired through the plugin: snapshot, input, wait_for over the shim;
      event push over `subscriptions/listen` (console, errors,
      navigation).
- [x] [Card 234](batch-cards/234-agent-control-end-to-end-proof.md) —
      packaged end-to-end proof: an MCP client drives the app unfocused;
      two clients interleave on one instance without interference;
      milestone and contract 022 evidence closeout.

## Acceptance

- Every contract 022 required-evidence item is satisfied or explicitly
  recorded as narrowed, with the contract amended to match.
- Untrusted-event limits (hover, OS drag-and-drop, `isTrusted`) are
  documented where consumers will read them.
- No native chrome is clicked anywhere in the proof; dialogs and menus go
  through commands.
