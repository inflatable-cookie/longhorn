# Grouped Custom-adapter Restore API

Status: checked private `0.1.0` API
Updated: 2026-08-03
Governing contract: [004](../contracts/004-configuration-storage-backup-and-recovery.md)

## Public Entry Points

`longhorn-config` exports these grouped protocol types:

- `BackupAdapterRestoreParticipation::GroupedFailureAtomic`
- `BackupAdapterStateEvidence::{Absent, Present}`
- `BackupAdapterGroupedRestore`
- `BackupAdapterGroupedStageRequest`
- `BackupAdapterRestoreStage`
- `BackupAdapterGroupedApplyRequest`
- `BackupAdapterGroupedApplyKind`
- `BackupAdapterGroupedVerifyRequest`
- `RestoreAdapterGroupPlan` and `RestoreAdapterGroupPlanEntry`
- `RestoreAdapterGroupReceiptEntry`
- `RestoreAdapterGroupExecutionOptions`, receipt, stage, and error
- `RestoreAdapterGroupRecoveryOutcome`, receipt, and error

`ConfigStore` supplies three host methods:

```rust
plan_grouped_adapter_restore(inspection, domains)
execute_grouped_adapter_restore(catalog, archive, inspection, plan, confirmation, options)
recover_grouped_adapter_restore(catalog, lock_timeout)
```

The plan digest covers the archive digest and every sorted member's domain,
adapter id, adapter confirmation, target evidence, and rollback evidence. A
different archive, selection, adapter, preview, current generation, or
confirmation fails before mutation.

## Adapter Contract

Inspection receives the verified archive `BackupSourceState` and returns
explicit target and current `BackupAdapterStateEvidence`. `Absent` carries no
digest. `Present` carries the adapter-defined semantic SHA-256 digest. Archive
absence must produce absent target evidence; archive presence must produce
present target evidence.

`stage` is side-effect free. It returns complete target and rollback payload
sets plus both explicit states. `Absent` requires zero payloads. `Present`
requires one or more payloads. Longhorn validates this shape, confined unique
paths, and per-domain/total byte limits before durably storing every byte.

`apply` receives either `Target` or `Rollback`, the persisted opaque payloads,
and exact expected state. It must publish that state and be safe to repeat
during recovery. `verify` receives the same kind and expected state, then
independently returns explicit observed state. An absent target is deletion;
an absent rollback state restores deletion. Neither uses a sentinel payload or
synthetic digest.

The ordinary single-domain API rejects grouped-only adapters. Archived absent
targets require grouped participation. Existing `Separate` and single-domain
`FailureAtomic` adapters retain their present-target behavior.

## Transaction And Recovery

The journal is private to the store coordination authority. Target publication
uses stable domain order; rollback unwinds in reverse order and verifies the
complete old generation. After a post-journal error, the terminal result is
fully committed, fully rolled back, or `RecoveryRequired` with all private
rollback evidence retained.

The version-2 grouped journal stores target and rollback evidence separately.
Unsupported versions and evidence/payload contradictions remain blocking.
Execution and recovery receipts return stable `RestoreAdapterGroupReceiptEntry`
values containing both states.

An interruption leaves the journal authoritative. Normal loads and mutations
fail closed. Boot code must register the exact descriptors and grouped adapter
catalogue, keep product authorities closed, and call
`recover_grouped_adapter_restore` before opening databases or services. The
recovery path needs no renderer, Tauri window, or product schema.

## Consumer Authority

Longhorn owns selection binding, bounds, private payload durability, journal
state, apply ordering, rollback, verification, and receipts. The consumer owns:

- app-wide shutdown and quiescence
- database connection and WAL policy
- exact domain selection and product meaning
- offline/boot orchestration
- restart scheduling and user presentation

Grouped restore is not exposed through the generic renderer config protocol.
Apps may add a narrow host operation only after their quiescence and restart
contract is explicit.
