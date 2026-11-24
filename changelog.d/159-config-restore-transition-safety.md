## Fixed

- Refuse backup capture and storage transitions while either relevant store has
  an unresolved restore journal. Check the transition journal again under the
  store guards and coordinate single-adapter restores.
- Bound transition and ordinary rollback reads, verify staged copy evidence
  before publication, and reject lossy transition paths and mismatched grouped
  rollback payload names.

## Breaking

- Rust exhaustive matches on `BackupCaptureError` and `StorageTransitionError`
  must handle `RestoreUnavailable`. Recover the affected store, then retry
  capture or create a fresh transition preview.
- Single-adapter restore can return `RestoreAdapterError::Coordination`.
  Handle the variant and retry a busy coordinator after it becomes available.
- Restore execution failures and adapter and recovery receipts now take
  `RestoreExecutionStageProjection`, `RestoreFailureTerminalProjection`,
  `RestoreAdapterOutcomeProjection`, and `RestoreRecoveryResultProjection`
  enum values in Rust. Use the matching variant in constructors and matches.
  TypeScript uses the corresponding literal unions. Existing documented JSON
  strings remain valid. Narrow arbitrary strings before assigning these fields.
