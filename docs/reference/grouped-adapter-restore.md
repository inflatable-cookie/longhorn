# Grouped Custom-adapter Restore API

Status: checked private `0.1.0` API
Updated: 2026-08-02
Governing contract: [004](../contracts/004-configuration-storage-backup-and-recovery.md)

## Public Entry Points

`longhorn-config` exports these grouped protocol types:

- `BackupAdapterRestoreParticipation::GroupedFailureAtomic`
- `BackupAdapterGroupedRestore`
- `BackupAdapterGroupedStageRequest`
- `BackupAdapterRestoreStage`
- `BackupAdapterGroupedApplyRequest`
- `BackupAdapterGroupedApplyKind`
- `BackupAdapterGroupedVerifyRequest`
- `RestoreAdapterGroupPlan` and `RestoreAdapterGroupPlanEntry`
- `RestoreAdapterGroupExecutionOptions`, receipt, stage, and error
- `RestoreAdapterGroupRecoveryOutcome`, receipt, and error

`ConfigStore` supplies three host methods:

```rust
plan_grouped_adapter_restore(inspection, domains)
execute_grouped_adapter_restore(catalog, archive, inspection, plan, confirmation, options)
recover_grouped_adapter_restore(catalog, lock_timeout)
```

The plan digest covers the archive digest and every sorted member's domain,
adapter id, adapter confirmation, target evidence, and current evidence. A
different archive, selection, adapter, preview, current generation, or
confirmation fails before mutation.

## Adapter Contract

`stage` is side-effect free. It returns complete target and rollback payload
sets plus semantic evidence. Empty rollback payloads may mean prior absence;
the evidence value makes that meaning explicit. Longhorn validates confined
unique paths and per-domain/total byte limits, then durably stores every byte
before it calls `apply`.

`apply` receives either `Target` or `Rollback`, the persisted opaque payloads,
and expected semantic evidence. It must publish exactly that state and be safe
to repeat during recovery. `verify` independently reads live authority and
returns its semantic digest or `None` for absence.

The ordinary single-domain API rejects grouped-only adapters. Existing
`Separate` adapters retain their separately confirmed and receipted behavior.

## Transaction And Recovery

The journal is private to the store coordination authority. Target publication
uses stable domain order; rollback unwinds in reverse order and verifies the
complete old generation. After a post-journal error, the terminal result is
fully committed, fully rolled back, or `RecoveryRequired` with all private
rollback evidence retained.

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
