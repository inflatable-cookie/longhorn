# 096 Nucleus Storage And Legacy Import

Status: ready
Owner: Tom
Roadmap: g01.014 batch 2
Governing refs: contracts 003, 004, and 012; Cards 094-095
Depends on: Card 095
Auto-start next card: no

## Objective

Move Nucleus desktop storage to canonical-id platform roots and import the
legacy `.nucleus` store without losing state or leaving two authorities.

## Repository Scope

- Nucleus desktop and its focused tests may change.
- Longhorn may receive conformance fixtures and migration evidence.
- Nucleus CLI storage policy remains unchanged.

## Scope

- canonical id `dev.nucleus.desktop`
- `platform-native-v1` default with no storage-name override
- `portable-v1` replacement for isolated proof roots
- fixed canonical-id locator
- typed config, data, state, cache, log, runtime, and backup roots
- `.nucleus` legacy discovery and explicit import
- SQLite native snapshot, ordinary file staging, locator-last commit
- separate window-placement and project-layout domains
- source retention, recovery, diagnostics, and cleanup deferral

## Steps

1. Recheck Nucleus worktree and pin the exact Card 095 private source receipts;
   verify them against its matching produced-artifact evidence.
2. Register canonical identity, default profile, portable proof profile, and
   fixed locator.
3. Register each current desktop store with its target class and adapter.
4. Add read-only `.nucleus` discovery and exact source inventory.
5. Split `ui.json` into window and project-layout migration outputs.
6. Stage ordinary files and invoke the SQLite snapshot adapter.
7. Verify target state, journal the transition, and commit the locator last.
8. Exercise interruption recovery and retain source plus cleanup receipts.
9. Remove the old normal-startup path authority and update focused docs/tests.

## Acceptance Criteria

- macOS, Windows, and Linux roots resolve from one selected profile
- default paths contain `dev.nucleus.desktop`, not display name or home dotfile
- explicit proof roots are absolute portable profiles and isolate every store
- SQLite, `ui.json`, snapshots, and drafts have explicit classes and adapters
- old schemas import idempotently with checksums and typed receipts
- the combined UI document seeds separate window and layout domains once
- failed or interrupted import returns to one last committed authority
- no normal startup silently reads, writes, merges, or deletes `.nucleus`
- desktop behavior and focused persistence tests pass

## Evidence Required

- three-platform path matrix
- missing, corrupt, future, overlap, and interrupted-import fixtures
- SQLite snapshot and ordinary-store receipts
- locator commit and recovery evidence
- before/after store inventory
- Nucleus lockfile and focused test report

## Stop Conditions

- a live SQLite database would be copied as ordinary files
- source and target roots overlap or cannot be distinguished
- current state cannot be split without an unresolved product decision
- another Nucleus writer bypasses the selected coordination authority
- donor worktree changes overlap the named storage files

## Next Task

Execute Card 097. Replace primary-window geometry mechanics while the new
window domain and retained legacy source provide exact rollback evidence.
