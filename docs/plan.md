# Plan

Updated: 2026-09-26

## Now

1. **Consumers adopt 0.2.2** (lane `longhorn-0-2-2-adoption`) —
   soundcheck-library, Loophole and Soundcheck share one Longhorn crate
   identity through path dependencies, so they move from `v0.1.0` to
   `v0.2.2` together, library first. Soundcheck also routes its two
   Rust-side pickers through the selection registry
   ([contract 022](knowledge/contracts/022-agent-app-control.md)).
   Consumer-owned work; Tom approved the writes on 2026-09-26.

## Next

- **Sweep removed records for rulings** — the 2026-09-26 cut removed the g02
  task files, delivery logs, handoffs and research memos. Rulings that live
  only there should be promoted into their owning knowledge file. Read them
  from Git history at `1182f622`.

## Not now

- **Remove the `dev` migration forward from `0.2.0`** — keep it until the next
  breaking release.
- **crates.io and hosted docs** — Rust crates stay `publish = false` and are
  taken by git tag ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md)).
