`longhorn_surfaces_config::load_or_default` no longer defaults past a restore.
While a restore is active or waiting for recovery it returns
`LayoutLoadError::Restore(UnavailableState)` instead of the registered default,
and it returns `LayoutLoadError::Store` where it returned `StoreError`. Migration:
match the new error type, resolve the restore (`ConfigStore::recover_restore`),
then load again. A missing optional authority still opens on the default.
