# Plan

Updated: 2026-09-26

## Now

1. **Release 0.3.0** (lane `release-0-3-0`) — the next release is `0.3.0`
   (operator ruling 2026-09-26), because raising the Svelte floor drops older
   Svelte. It carries:
   - `longhorn-poodle-svelte`'s Svelte peer at `>=5.56.8 <6`
     ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md#workspace-and-versions));
   - removal of the temporary `dev` feature on `longhorn-tauri-agent-control`
     (no known consumer still uses it);
   - a local guard: `qa` fails when a release gate needs a tool `release.yml`
     does not install.
   - a full refresh of every dependency to its newest compatible release,
     in one sweep (operator ruling 2026-09-26). Poodle refreshes and
     releases first, so the sweep takes the new Poodle pin.

## Next

- **Consumer apps take 0.3.0** — after Longhorn 0.3.0 ships and Underlay's
  matching dependency refresh lands, schedule upgrades across every
  consuming app together (Longhorn's desktop apps and Underlay's web apps).
  0.3.0 is breaking, so each app needs its own move.

## Not now

- **Bulk fork deletion** (delete-many by predicate) — waits for field
  evidence that deleting forks one at a time is not enough. No clock-based
  predicate: the host computes such sets from its own `recorded_at` stamps.
- **crates.io and hosted docs** — Rust crates stay `publish = false` and are
  taken by git tag ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md)).
