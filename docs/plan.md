# Plan

Updated: 2026-09-26

## Now

1. **Choose what follows 0.2.1** — Q-001 in
   [questions](knowledge/questions.md) is open with the operator.
   Recommended: a small hardening lane, then `0.2.2` with
   `effigy release:bump`. The lane covers three `PAPERCUTS.md` entries: a
   drift check between the two agent-control shim copies
   (`crates/longhorn-tauri-agent-control/src/agent_control_shim.js` and
   `packages/longhorn/src/agent-control/shim.ts`), the `set_file_input` size
   cap enforced in the Tauri handler, and `release:bump` reporting fixes.
   `main` already carries unreleased Rust-side selection and
   `set_file_input`.

## Next

- **Soundcheck adopts both picker routes** (Rust-side selection and
  `set_file_input`) — consumer-owned; Soundcheck writes need the operator's
  go-ahead.
- **Sweep removed records for rulings** — the 2026-09-26 cut removed the g02
  task files, delivery logs, handoffs and research memos. Rulings that live
  only there should be promoted into their owning knowledge file. Read them
  from Git history at `1182f622`.

## Not now

- **Remove the `dev` migration forward from `0.2.0`** — keep it until the next
  breaking release.
- **crates.io and hosted docs** — Rust crates stay `publish = false` and are
  taken by git tag ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md)).
