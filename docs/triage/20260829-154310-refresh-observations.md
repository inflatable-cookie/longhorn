# Refresh observations — 2026-08-29

Status: open
Owner: Longhorn maintainer
Next check: the next docs cleanup pass or a change to handoff retention policy

## Observations

- Longhorn had no target-local `effigy check:agent-instructions` selector; the
  consumer-safe installed-Northstar fallback was required for this refresh
  (closed in papercuts wave 14 — `AGENTS.md` now names the command).
- The root and docs README state summaries lagged the generation index and
  latest logs; this refresh updates them.
- `docs/handoffs/` retains older worker dispatch artifacts with
  `ready-to-launch` or `ready` status after their related lanes closed.

## Disposition

- README drift: reworked in this refresh.
- Agent-instruction selector gap: closed by documenting the installed-Northstar
  command on `AGENTS.md` (papercuts wave 14). No target-local Rhai copy.
- Handoff retention: keep open. Do not delete or rehome these artifacts without
  owner confirmation.
