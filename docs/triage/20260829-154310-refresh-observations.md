# Refresh observations — 2026-08-29

Status: open
Owner: Longhorn maintainer
Next check: the next docs cleanup pass or a change to handoff retention policy

## Observations

- Longhorn has no target-local `effigy check:agent-instructions` selector. The
  consumer-safe installed-Northstar fallback was required for this refresh.
- The root and docs README state summaries lagged the generation index and
  latest logs; this refresh updates them.
- `docs/handoffs/` retains older worker dispatch artifacts with
  `ready-to-launch` or `ready` status after their related lanes closed.

## Disposition

- README drift: reworked in this refresh.
- Agent-instruction selector gap: keep open. The immediate friction is recorded
  in `PAPERCUTS.md`; a target-local alias or explicit pointer remains a later
  maintenance choice.
- Handoff retention: keep open. Do not delete or rehome these artifacts without
  owner confirmation.
