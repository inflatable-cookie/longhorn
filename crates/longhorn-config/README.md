# longhorn-config

Versioned configuration domains, cross-platform storage policy, coordinated
atomic mutation, bounded backup, and journaled restore.

## Grouped Custom-adapter Restore

Use grouped restore when several adapter-owned authorities must move as one
failure-atomic generation. Each participating adapter declares
`BackupAdapterRestoreParticipation::GroupedFailureAtomic` and returns a
`BackupAdapterGroupedRestore` extension.

The extension has three operations:

- `stage`: produce opaque target and exact rollback payloads without live mutation
- `apply`: publish one already-durable target or rollback payload set
- `verify`: observe current semantic evidence independently

`BackupAdapterStateEvidence` represents both target and rollback state as
`Absent` or `Present { sha256 }`. Inspection receives the verified archive
`BackupSourceState`. Absent state always carries zero payloads; present state
always carries at least one. Apply and verify requests include `Target` or
`Rollback` plus the exact expected state.

`ConfigStore::plan_grouped_adapter_restore` binds one inspected archive and
the exact sorted domain set to one confirmation digest.
`ConfigStore::execute_grouped_adapter_restore` re-inspects and stages every
domain, persists all private payloads and one journal, applies and verifies the
whole set, and rolls the whole set back on failure.
`ConfigStore::recover_grouped_adapter_restore` is the renderer-free boot path.
It requires the exact registered descriptors and adapter catalogue used by the
journal.

Execution and recovery receipts expose stable per-domain target and rollback
evidence through `RestoreAdapterGroupReceiptEntry`.

Consumers must quiesce every external authority before execution and recovery.
Longhorn does not close databases, stop services, schedule restart, select
product domains, or interpret adapter payloads.

Repository references: `docs/reference/grouped-adapter-restore.md` and
`docs/guides/storage-configuration-backup.md`.
