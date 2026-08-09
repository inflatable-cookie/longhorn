# Platform Storage Layout Profiles

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added immutable canonical application identity with one optional stable
  storage name
- added pure macOS, Windows, and Linux platform-directory facts
- added versioned native, unified-root, and portable layout profiles
- added typed config, data, state, cache, runtime, log, backup, workspace,
  policy, and project roots
- added per-root and leaf provenance, visible lifecycle warnings, diagnostic
  projection, and deterministic layout digest
- split machine state from durable data in `StorageRoots`
- added durable, state, and rebuildable database-directory conventions
- added a dependency-light adapter for paths supplied by Tauri applications

## Identity And Layout

The canonical application id is the default leaf on every platform. One
explicit stable storage name replaces it throughout native and unified
layouts. Display name, executable name, current directory, and environment are
not inputs. Invalid identity fails before resolution.

`platform-native-v1` preserves platform lifecycle roots.
`unified-app-root-v1` creates typed children below the native durable app root
and reports its cache, runtime, and backup consequences. `portable-v1`
requires one explicit absolute root and never guesses a location.

The pure resolver creates no directory and reads no ambient state. Its receipt
binds profile, target platform, canonical identity, effective leaf,
provenance, paths, and warnings into a stable SHA-256 digest.

## Tauri Boundary

A direct Tauri crate dependency was characterized and rejected. Its current
transitive graph raised the workspace floor above Rust 1.85 and coupled the
adapter to one Tauri patch line.

`longhorn-tauri-config` instead accepts a small platform snapshot populated
from the consumer's Tauri application path API. It maps those raw bases into
Longhorn facts without importing Tauri or resolving an app leaf. Linux state
remains an explicit XDG fact because Tauri has no state-directory API.

## Evidence

- exact native path matrices pass for macOS, Windows, and Linux
- unified and portable matrices pass on all three targets
- Soundcheck, Loophole, Split-shell, and Nucleus identity fixtures pass
- missing, empty, relative, invalid identity, and invalid profile inputs fail
  typed
- root overrides, lifecycle warnings, provenance, database placement,
  diagnostics, and digest behavior pass
- Tauri path-snapshot mapping passes for all three target policies
- 110 Rust tests passed
- Rust 1.85 workspace check passed
- stable Clippy passed with warnings denied
- Effigy QA passed; Doctor reports 17 size warnings and zero errors

## Boundary

No profile locator, storage migration, legacy import, filesystem mutation,
database I/O, consumer write, settings UI, TypeScript, Svelte, or Poodle
dependency was added.

## Posture

`strict-ready`

Card 011 is complete. Card 009 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start card 009 for the optional age v1 archive adapter.
