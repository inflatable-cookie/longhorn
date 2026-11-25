Rust exhaustive matches on `BackupCaptureError` and `StorageTransitionError`
must handle `RestoreUnavailable`. Recover the affected store, then retry
capture or create a fresh transition preview.
