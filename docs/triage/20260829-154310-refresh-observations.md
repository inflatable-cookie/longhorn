# Handoff retention — refresh observation

Status: open
Owner: Longhorn maintainer
Next check: the next docs cleanup pass or a change to handoff retention policy

`docs/handoffs/` retains older worker dispatch artifacts with
`ready-to-launch` or `ready` status after their related lanes closed. Keep
open: do not delete or rehome these artifacts without owner confirmation.

Opened 2026-08-29 as a refresh observation. The README state drift and the
agent-instruction selector gap from that refresh were resolved; this is the
unresolved remainder.
