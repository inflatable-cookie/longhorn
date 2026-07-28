# g01 Shared Desktop Foundation And Adoption

## Generation Runway

| Milestone | State | Outcome |
| --- | --- | --- | --- |
| [g01.001](001-foundation-contracts-and-package-topology.md) | complete | contracts and package graph |
| [g01.002](002-configuration-backup-and-recovery.md) | active | versioned domains, safe writes, backup, restore |
| [g01.003](003-display-geometry-and-window-planning.md) | blocked | pure display, coordinates, geometry, window plans |
| [g01.004](004-tauri-window-host-and-lifecycle.md) | blocked | native window apply and lifecycle |
| [g01.005](005-layout-container-region-and-panel-core.md) | blocked | Surface-independent layout state |
| [g01.006](006-optional-surfaces-and-cross-window-drag.md) | blocked | optional full hosting and transfer |
| [g01.007](007-typescript-svelte-poodle-and-app-shell.md) | blocked | checked clients and thin UI adapters |
| [g01.008](008-settings-registry-and-shell.md) | blocked | centralized composable settings |
| [g01.009](009-typed-bridge-and-optional-backend-topology.md) | researchable | direct/local/remote semantic seam |
| [g01.010](010-command-registry-keymaps-and-palette.md) | blocked | commands, input, keymaps, palette |
| [g01.011](011-history-kernel-and-branching-prototype.md) | researchable | proven linear kernel, fork decision |
| [g01.012](012-async-operations-and-notifications.md) | incubation | jobs, progress, cancellation, notifications |
| [g01.013](013-native-content-islands-prototype.md) | prototype | child webview/native/render host seam |
| [g01.014](014-nucleus-no-surface-migration.md) | blocked | first simple consumer |
| [g01.015](015-loophole-full-hosting-migration.md) | blocked | advanced full-stack consumer |
| [g01.016](016-secondary-consumers-and-greenfield-release.md) | blocked | Soundcheck, Bovine, Jetstream, first release |

## Dependency Shape

```text
001 contracts/package graph
 ├─ 002 configuration ─┬─ 003 display/window plan ─ 004 Tauri window host
 │                     ├─ 005 layout core ─ 006 optional Surfaces/drag
 │                     └─ 008 settings
 ├─ 009 bridge/topology ─ 010 commands/input/palette
 ├─ 011 history research
 ├─ 012 async operations research
 └─ 013 native islands prototype

004-010 ─ 014 Nucleus ─ 015 Loophole ─ 016 secondary consumers/release
```

Research/prototype work in 009, 011, 012, and 013 may run beside foundation
implementation after their named contract questions are bounded. Promotion,
not research activity, gates dependent implementation.

## Active Milestone

`g01.002 Configuration, Backup, And Recovery`

Domain storage, coordinated atomic mutation, bounded debounce, and explicit
flush are complete. No implementation card is ready.
[Card 004](batch-cards/004-backup-archive-and-restore-contract.md) records the
backup/archive contract gate.

## Milestones

The complete known g01 suite is compiled above. Milestones are planning
envelopes, not execution authority. Card 004 is a paused planning gate.

## Next Task

Research and promote the backup archive, encryption, snapshot, and atomic
restore protocol named in card 004.
