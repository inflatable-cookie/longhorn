# Questions

Questions that block or shape work. Reference them by ID from the plan and from
briefs. An answered question keeps only its pointer to where the answer lives.

## Q-001 — What follows the 0.2.1 release?

Status: open
Options: a hardening lane (shim drift check, `set_file_input` cap in the Tauri
handler, `release:bump` reporting fixes, all in `PAPERCUTS.md`) then `0.2.2`;
cut `0.2.2` now; or a Soundcheck adoption lane (consumer writes need the
operator's go-ahead). See [plan](../plan.md).

## Q-002 — How are Rust-side pickers and HTML file inputs admitted to agent control?

Status: answered 2026-09-26
Answer: [contract 022](contracts/022-agent-app-control.md) — a consumer-carried
origin for Rust pickers; inline base64 bytes with an 8 MiB cap and no path
reads for file inputs.

## Q-003 — Does contract 023 carry the production MCP role?

Status: answered 2026-09-22
Answer: No. [contract 023](contracts/023-production-contextual-agent-tool-boundary.md)
withdrawal; the contract 022 opt-in server is the production MCP.
