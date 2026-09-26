# Plan

Updated: 2026-09-26

## Now

1. **Release gates cannot need a tool the runner lacks** — local `qa` fails
   when a release gate needs a tool `release.yml` does not install. The
   0.2.2 dry run failed on `cargo deny`; the runner is fixed, the guard is
   not.
2. **Svelte peer range meets Poodle's** — `longhorn-poodle-svelte` moves to
   `>=5.56.8 <6` in the next release
   ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md#workspace-and-versions)).
   Raising the floor drops older Svelte, so the next release may be `0.3.0`;
   if it is, the `dev` forward from `0.2.0` goes in the same release.

## Not now

- **Bulk fork deletion** (delete-many by predicate) — waits for field
  evidence that deleting forks one at a time is not enough. No clock-based
  predicate: the host computes such sets from its own `recorded_at` stamps.
- **Remove the `dev` migration forward from `0.2.0`** — keep it until the next
  breaking release.
- **crates.io and hosted docs** — Rust crates stay `publish = false` and are
  taken by git tag ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md)).
