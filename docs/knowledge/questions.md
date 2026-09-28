# Questions

Questions that block or shape work. Reference them by ID from the plan and from
briefs. An answered question keeps only its pointer to where the answer lives.

## Q-001 — What follows the 0.2.1 release?

Status: answered 2026-09-26
Answer: a hardening lane, then `0.2.2`. Soundcheck adoption waits until
Soundcheck finishes its own Northstar cutover. See lane:release-0-3-0.

## Q-002 — How are Rust-side pickers and HTML file inputs admitted to agent control?

Status: answered 2026-09-26
Answer: [contract 022](contracts/022-agent-app-control.md) — a consumer-carried
origin for Rust pickers; inline base64 bytes with an 8 MiB cap and no path
reads for file inputs.

## Q-003 — Does contract 023 carry the production MCP role?

Status: answered 2026-09-22
Answer: No. [contract 023](contracts/023-production-contextual-agent-tool-boundary.md)
withdrawal; the contract 022 opt-in server is the production MCP.

## Q-004 — Should the Svelte tier import or generate Poodle spec shapes?

Status: open
Context: the Svelte tier re-declares Poodle's `StatusTone` and `ProgressSpec`
as `OperationStatusTone` and `OperationProgressView`, so a new Poodle member
does not reach the Svelte projector. The 2026-08-09 recommendation was to
import from `@inflatable-cookie/poodle-svelte` where the type is exported,
and generate from `poodle-specs` where it is not. Poodle-side work; it blocks
nothing. Owner file:
[cross-backend projection](architecture/cross-backend-projection.md).

## Q-005 — Should the fork tree leave the held-surface register?

Status: answered 2026-09-28
Answer: yes. The fork tree (`longhorn-history-tree`, its Tauri host, and
`@inflatable-cookie/longhorn/history-tree`) is a selectable optional system
on top of linear history, governed by
[contract 008](contracts/008-history-kernel-boundary.md). Loophole already
uses it in production.
