# Research Master Index

Status: active  
Owner: Tom  
Updated: 2026-07-31

## Architecture Area To Research

| Area | Primary memo | Promotion |
| --- | --- | --- |
| cross-app extraction | [Initial Tauri Audit](translation-memos/001-tauri-application-extraction-audit.md) | `../architecture/system-architecture.md` |
| workspace composition | [Initial Tauri Audit](translation-memos/001-tauri-application-extraction-audit.md) | `../contracts/002-composable-workspace-hosting.md` |
| migration discipline | [Initial Tauri Audit](translation-memos/001-tauri-application-extraction-audit.md) | `../contracts/003-extraction-and-consumer-migration.md` |
| configuration, settings, topology, commands | [Shared Desktop Systems Follow-up](translation-memos/002-shared-desktop-systems-follow-up.md) | `../contracts/004-configuration-storage-backup-and-recovery.md` through `../contracts/007-optional-backend-topology.md` |
| history kernel and fork boundary | [History Kernel And Fork Boundary](translation-memos/015-history-kernel-and-fork-boundary.md) | `../architecture/system-architecture.md`; `../architecture/package-topology.md`; `../contracts/008-history-kernel-boundary.md`; `../roadmaps/g01/011-history-kernel-and-branching-prototype.md` |
| display, window, IPC, drag, lifecycle, packages | [Foundation Boundary Characterization](translation-memos/003-foundation-boundary-characterization.md) | `../contracts/009-display-identity-coordinates-and-window-planning.md` through `../contracts/013-svelte-and-poodle-adapter-lifecycle.md`; `../architecture/package-topology.md` |
| configuration coordination and atomic mutation | [Configuration Coordination And Atomic Mutation](translation-memos/004-configuration-coordination-and-atomic-mutation.md) | `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/002-coordinated-atomic-configuration-mutation.md` |
| debounced mutation and explicit flush | [Debounced Mutation And Explicit Flush](translation-memos/005-debounced-mutation-and-explicit-flush.md) | `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/003-debounced-mutation-and-explicit-flush.md` |
| backup archive, encryption, and restore | [Backup Archive, Encryption, And Restore](translation-memos/006-backup-archive-encryption-and-restore.md) | `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/005-backup-inventory-and-consistent-snapshot.md` through card 010 |
| cross-platform storage locations and profiles | [Cross-platform Storage Layout Profiles](translation-memos/007-cross-platform-storage-layout-profiles.md) | `../architecture/package-topology.md`; `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/011-platform-storage-layout-profiles.md`; card 012 |
| Tauri window host and lifecycle | [Tauri Window Host And Lifecycle](translation-memos/008-tauri-window-host-and-lifecycle.md) | `../contracts/009-display-identity-coordinates-and-window-planning.md`; `../roadmaps/g01/004-tauri-window-host-and-lifecycle.md`; cards 017-022 |
| layout container, region, and panel core | [Layout Core Boundary Characterization](translation-memos/009-layout-core-boundary-characterization.md) | `../architecture/system-architecture.md`; `../contracts/014-layout-container-region-and-panel-core.md`; `../roadmaps/g01/005-layout-container-region-and-panel-core.md`; cards 023-027 |
| optional Surface hosting and cross-window transfer | [Surface Hosting And Transfer Boundary](translation-memos/010-surface-hosting-and-transfer-boundary.md) | `../architecture/system-architecture.md`; `../contracts/002-composable-workspace-hosting.md`; `../contracts/011-cross-window-transfer.md`; `../roadmaps/g01/006-optional-surfaces-and-cross-window-drag.md`; cards 028-035 |
| client, Svelte, Poodle, drag, and shell adapters | [Client, Svelte, Poodle, And Shell Boundary](translation-memos/011-client-svelte-poodle-and-shell-boundary.md) | `../architecture/package-topology.md`; `../contracts/012-distribution-and-compatibility.md`; `../contracts/013-svelte-and-poodle-adapter-lifecycle.md`; `../roadmaps/g01/007-typescript-svelte-poodle-and-app-shell.md`; cards 036-041 |
| settings registry, transactions, and shell | [Settings Registry And Transaction Boundary](translation-memos/012-settings-registry-and-transaction-boundary.md) | `../architecture/system-architecture.md`; `../architecture/package-topology.md`; `../contracts/005-settings-and-system-registration.md`; `../roadmaps/g01/008-settings-registry-and-shell.md`; cards 042-048 |
| typed bridge and optional backend topology | [Typed Bridge And Backend Topology Boundary](translation-memos/013-typed-bridge-and-backend-topology-boundary.md) | `../architecture/system-architecture.md`; `../architecture/package-topology.md`; `../contracts/007-optional-backend-topology.md`; `../contracts/010-rust-typescript-ipc-and-events.md`; `../roadmaps/g01/009-typed-bridge-and-optional-backend-topology.md` |
| command registry, keyboard, keymaps, and palette | [Command, Input, And Palette Boundary](translation-memos/014-command-input-and-palette-boundary.md) | `../architecture/system-architecture.md`; `../architecture/package-topology.md`; `../contracts/006-command-action-and-input.md`; `../roadmaps/g01/010-command-registry-keymaps-and-palette.md`; cards 056-061 |
| async operations and notifications | [Async Operation And Notification Boundary](translation-memos/016-async-operation-and-notification-boundary.md) | `../architecture/system-architecture.md`; `../architecture/package-topology.md`; `../contracts/015-async-operation-lifecycle.md`; `../contracts/016-notification-ledger-and-projection.md`; `../roadmaps/g01/012-async-operations-and-notifications.md`; cards 075-081 |
| native content islands | [Native Content Island Boundary](translation-memos/017-native-content-island-boundary.md) | `../architecture/system-architecture.md`; `../architecture/package-topology.md`; `../contracts/017-native-content-island-coordination.md`; `../roadmaps/g01/013-native-content-islands-prototype.md` |
| workspace integrity audit | [Workspace Integrity Audit](translation-memos/018-workspace-integrity-audit.md) | `../roadmaps/g02/README.md`; cards 138-148 |
| application update and release channels | [Application Update And Release Channels](translation-memos/019-application-update-and-release-channels.md) | `../contracts/018-application-update-and-release-channels.md`; `../roadmaps/g02/009-application-update-and-release-channels.md`; cards 150-154 |
| licensing, entitlement, and activation | [Licensing, Entitlement, And Activation](translation-memos/020-licensing-entitlement-and-activation.md) | `../contracts/019-licensing-entitlement-and-activation.md`; `../roadmaps/g02/010-licensing-entitlement-and-activation.md`; cards 155-158 |

## Open Research

- non-macOS strong display evidence and ambiguity UX
- non-macOS packaged cross-window transfer across platforms and display scales
- cross-document panel transaction and copy-transfer authority
- production service transport, discovery, authentication, and endpoint policy
- durable offline mutation and server-synchronized transaction authority
- macros, extended input triggers, native accelerators, and synchronized
  keymaps
- public registry name verification
- minisign key custody and rotation: only one public key is embedded per
  build, so rotation means shipping a version that accepts the successor,
  waiting for adoption, then switching. Key loss strands every install
  permanently. Operator-owned.
- update rollback: Tauri has no mechanism. Staged rollout limits blast
  radius but does not undo a bad release.
- signing key custody and rotation for consumer-issued offline licences:
  the same one-embedded-key problem as the updater's minisign key. The two
  should be solved once, together. Consumer-owned.
- reinstall-farm detection via coarse hardware fingerprinting: deliberately
  deferred, trading privacy against abuse not yet observed.
- branch reference, checkpoint, pruning, migration, and performance decision
  after the private history-tree prototype
- packaged native-content proof across display scales and supported platforms
- production child-webview, isolated-window, and backing-surface adapter split
