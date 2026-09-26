# Plan

Updated: 2026-09-26

## Now

1. **Harden, then release 0.2.2** (lane `hardening-0-2-2`) — close three
   gaps found in review before the unreleased agent-control work ships
   (Q-001). The `set_file_input` 8 MiB cap holds for every caller, not only
   the MCP edge. The Tauri shim copy cannot drift from its TypeScript source.
   `effigy release:bump` reports every file it changes and never leaves a
   partial bump. Then cut `0.2.2` through
   [release.md](knowledge/contracts/release.md). `main` already carries
   Rust-side selection and `set_file_input`.

## Next

- **Soundcheck adopts both picker routes** (Rust-side selection and
  `set_file_input`) — waits for Soundcheck to finish its Northstar cutover.
  Consumer-owned; Soundcheck writes need the operator's go-ahead.
- **Sweep removed records for rulings** — the 2026-09-26 cut removed the g02
  task files, delivery logs, handoffs and research memos. Rulings that live
  only there should be promoted into their owning knowledge file. Read them
  from Git history at `1182f622`.

## Not now

- **Remove the `dev` migration forward from `0.2.0`** — keep it until the next
  breaking release.
- **crates.io and hosted docs** — Rust crates stay `publish = false` and are
  taken by git tag ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md)).
