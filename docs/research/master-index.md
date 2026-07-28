# Research Master Index

Status: active  
Owner: Tom  
Updated: 2026-07-28

## Architecture Area To Research

| Area | Primary memo | Promotion |
| --- | --- | --- |
| cross-app extraction | [Initial Tauri Audit](translation-memos/001-tauri-application-extraction-audit.md) | `../architecture/system-architecture.md` |
| workspace composition | [Initial Tauri Audit](translation-memos/001-tauri-application-extraction-audit.md) | `../contracts/002-composable-workspace-hosting.md` |
| migration discipline | [Initial Tauri Audit](translation-memos/001-tauri-application-extraction-audit.md) | `../contracts/003-extraction-and-consumer-migration.md` |
| configuration, settings, topology, commands | [Shared Desktop Systems Follow-up](translation-memos/002-shared-desktop-systems-follow-up.md) | `../contracts/004-configuration-storage-backup-and-recovery.md` through `../contracts/007-optional-backend-topology.md` |
| history | [Shared Desktop Systems Follow-up](translation-memos/002-shared-desktop-systems-follow-up.md) | `../contracts/008-history-kernel-boundary.md` |
| display, window, IPC, drag, lifecycle, packages | [Foundation Boundary Characterization](translation-memos/003-foundation-boundary-characterization.md) | `../contracts/009-display-identity-coordinates-and-window-planning.md` through `../contracts/013-svelte-and-poodle-adapter-lifecycle.md`; `../architecture/package-topology.md` |
| configuration coordination and atomic mutation | [Configuration Coordination And Atomic Mutation](translation-memos/004-configuration-coordination-and-atomic-mutation.md) | `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/002-coordinated-atomic-configuration-mutation.md` |
| debounced mutation and explicit flush | [Debounced Mutation And Explicit Flush](translation-memos/005-debounced-mutation-and-explicit-flush.md) | `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/003-debounced-mutation-and-explicit-flush.md` |
| backup archive, encryption, and restore | [Backup Archive, Encryption, And Restore](translation-memos/006-backup-archive-encryption-and-restore.md) | `../contracts/004-configuration-storage-backup-and-recovery.md`; `../roadmaps/g01/batch-cards/005-backup-inventory-and-consistent-snapshot.md` through card 010 |

## Open Research

- non-macOS strong display evidence and ambiguity UX
- packaged cross-window transfer across platforms and display scales
- server-synchronized settings and remote transaction authority
- backup and restore settings UX
- public registry name verification
- generic history payload plus branch/checkpoint performance
- async operation and notification lifecycle
- common native-content-island contract
