# Plan

Updated: 2026-09-26

## Now

Nothing is ready. Every known consumer app is on `0.2.2`.

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
