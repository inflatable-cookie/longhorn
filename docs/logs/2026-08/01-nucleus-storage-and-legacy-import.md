# Nucleus Storage And Legacy Import

Date: 2026-08-01
Card: 096
Outcome: `pass`

## What Changed

- Changed the desktop application id to `com.inflatablecookie.nucleus`.
- Adopted Longhorn `platform-native-v1` roots with no storage-name override.
- Replaced the old proof-root switch with explicit `portable-v1` selection.
- Added fixed-locator bootstrap inspection and fail-closed recovery handling.
- Imported `.nucleus` through UI split, SQLite online backup, snapshot-tree,
  and draft-tree adapters.
- Split runtime window placement from project layout persistence.
- Removed normal-startup reads and writes through the old desktop root.
- Retained the legacy root, SQLite WAL, and unknown files. Cleanup is deferred.

## Evidence

Nucleus commit `1c0f8e225849718888afdba63cee4745f623e906` consumes Longhorn commit
`973c89f260804a777747ead3218b50d146d16118`. The prior admitted Nucleus
receipt is `df5af3da03f8392f948ff65d0a3cf16c36cb6c12`.
The committed Nucleus lockfile SHA-256 is
`608ebcfad93e417fb3c25a4b907a76c61678f47cdc58d6f5d4c97f26c819ea51`.

The cutover fixture records macOS, Windows, and Linux roots; four migrated
domains; locator-last authority; retained-source posture; and failure cases.
`effigy proof:nucleus-storage-cutover` reads the pinned commit, not the live
worktree. An unrelated `TerminalPanel.svelte` edit remained untouched.

Nucleus validation passed:

- 70 Rust tests
- 40 renderer tests
- four focused migration tests, including a live open WAL
- Rust workspace check and formatting
- desktop Svelte check with one pre-existing ARIA warning
- Northstar posture check

Longhorn transition tests provide overlap rejection plus injected recovery
before and after locator publication. Nucleus supplies missing-root,
corrupt/future UI, occupied-target, live-WAL, split-store, locator, receipt,
and retained-source coverage.

## Decision Notes

The canonical identity corrects the earlier provisional
`dev.nucleus.desktop` target. Historical donor evidence keeps the old Tauri id
where it describes the pre-cutover app; current target docs and fixtures use
`com.inflatablecookie.nucleus`.

Package-manager publication remains deferred. Card 097 can consume the new
window-placement store without changing storage authority.
