# Agent Control Spike And Promotion

Date: 2026-08-19
Scope: memo 024, contract 022, g02.029 (Card 227, PR 2), g02.030 compilation

## What happened

The agent app-control lane went from problem statement to promoted
contract in one day. Memo 024 settled the protocol by research (CDP
rejected on input-fidelity and client-compatibility grounds; stateless MCP
over streamable HTTP adopted), contract 022 was drafted, and Card 227 ran
the two probes documentation could not answer. A worker built the spike in
`prototypes/agent-control/` and delivered PR 2 with wire-level evidence.

## Evidence highlights

- rmcp 3.1.3 `StreamableHttpService` mounted statelessly on axum inside a
  Tauri app process. Claude Code 2.1.235 negotiated revision 2026-07-28
  via `server/discover` with zero `Mcp-Session-Id` on the wire; malformed
  requests drew explicit `-32020` header errors. rmcp supports 2026-07-28
  but defaults to 2025-11-25 — a memo correction.
- `WKWebView.takeSnapshot` returned DOM-fresh images frontmost, unfocused,
  fully occluded, and minimized, judged by bracketing each PNG with
  `evaluate` reads of the page counter. No permission prompt, no private
  API. Another-Space was not probed and is recorded as such.
- New contract-relevant fact: WKWebView coalesces DOM timers in every
  window state and stops `requestAnimationFrame` while the window is not
  key. Contract 022's `wait_for` is therefore DOM-relative by rule; no
  time-only or rAF wait exists in the vocabulary.

## Promotion

Contract 022 draft → active with the rAF/timer caveat folded into
`wait_for` and `screenshot`. Memo 024 complete and promoted. g02.029
complete; g02.030 compiled to ready (Card 228 core crate, Card 229
stateless server); 031-032 planned with reserved card numbers 230-234.

## Worker loop

First orchestrator-dispatched worker lane in this repo: handoff
`docs/handoffs/20260819-141558-agent-control-spike-worker.md`, worktree
`~/Dev/worktrees/longhorn-227`, PR 2 reviewed against the card and merged
on operator authorisation. Scope held exactly; the two non-blocking review
notes (borrowed example icon, uncommitted probe script) carry no follow-up
card.
