# Coordinated Atomic Configuration Mutation

Date: 2026-07-28  
State: complete implementation batch

## Outcome

- added explicit canonicalized coordination authorities
- added stable `.longhorn/config.lock` files
- layered one process-local mutex per authority over `fs4` advisory locking
- added finite busy/timeout acquisition and crash-safe release
- added current-value encoding to consumer domain codecs
- added patch mutation over an authoritative reread inside the lock
- added capability-relative unique temporary creation, file sync, atomic
  rename, cleanup, and durability receipts
- defaulted new Unix domain files and lock files to mode `0600`
- refused unsafe load outcomes and unproven project-shared authorities

## Public Boundary

`ConfigStore::new` now requires both storage roots and a
`CoordinationAuthority`. `ConfigStore::mutate` requires explicit finite lock
timeout and durability policy. It accepts a typed patch closure and never a
blind encoded replacement.

Successful receipts distinguish file-synced publication from file-and-directory
sync. A durable request that fails after rename returns a typed publication
failure with `published: true`.

Migrated-in-memory values remain non-writable until pre-migration backup exists.
Recovery, future, read-only, unavailable, and project-shared authorities remain
preserved and refused.

## Evidence

- 34 passing tests
- two store instances preserve unrelated field updates
- two helper processes serialize fresh-value patches
- finite cross-process timeout and killed-holder recovery
- persistent unlocked lock-file proof
- complete old-or-new reader loop
- every injected pre-rename stage preserves the old target
- unpublished temporary cleanup
- post-publication durability-failure reporting
- Unix permission and parent-symlink confinement fixtures
- Rust 1.85 workspace check
- clean format, clippy, Effigy doctor, test plan, and full QA

## Platform Boundary

The batch was executed on macOS. The implementation uses `fs4`'s Windows lock
adapter and `cap-std` replacement path, but packaged Windows durability remains
later release evidence. Unsupported parent-directory synchronization is
reported as file-only durability or a typed post-publication failure.

## Posture

`strict-paused`

Card 002 is complete. Card 003 records the missing generic debounce/flush
scheduling contract. It is not executable.

## Next

Research donor debounce behavior and promote coalescing, scheduler ownership,
retry, shutdown, and receipt semantics.
