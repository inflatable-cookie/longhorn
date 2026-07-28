# Restore Inspection, Planning, And Staging

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added side-effect-free restore inspection over verified plaintext archives
- added exact per-domain conflict choices and confirmation-bound plans
- added capability-scoped current-file evidence and canonical plan digests
- added coordinated stale recheck and complete private current-schema staging
- kept all live publication, rollback, and recovery work out of this batch

## Capability Boundary

`ConfigStore` continues to own registered descriptors only. The borrowed
`BackupCatalog` supplies consumer schema, migration, codec, and validation
behavior for one inspection or staging call. This keeps app schemas out of
Longhorn authority and prevents a descriptor-only registration from becoming
an implicit restore capability.

Inspection compares stable application id and producer name. Version
differences remain visible in the manifest but do not block schema-driven
restore. Reports retain every included domain and exclusion exactly once.
They distinguish unknown domains, target-policy exclusion, unavailable
ordinary storage, custom adapter requirements, source-preserved evidence,
future or corrupt input, missing migration, and target preparation failure.

## Plan

Every included manifest domain requires one explicit choice:

- use archive state
- keep current state

Missing and unexpected choices are typed errors. An incompatible selected
domain fails the whole plan; it is never silently skipped.

Selected current files are hashed through their registered root capability.
Evidence records absence or exact byte length and SHA-256. The canonical
confirmation digest binds the complete archive hash, every conflict choice,
derived create/replace/delete/migrate/unchanged action, current evidence, and
the prepared target schema and digest.

## Staging

Preparation acquires the store coordinator and rereads every selected current
target. Any evidence change returns a typed stale-plan failure before staging
is returned.

Every selected present payload is migrated again from unchanged archive bytes,
decoded, validated, re-encoded into the current registered envelope, and
validated raw. The staged target must match the target evidence confirmed in
the plan. One failure prevents the complete opaque staging set. Absent and
unchanged targets remain explicit. No filesystem publication occurs.

## Evidence

- seven focused restore acceptance tests
- Rust 1.85 workspace tests: 85 passed
- installed stable Clippy with warnings denied
- rustdoc with warnings denied
- Effigy full QA
- Effigy Doctor: 15 checks passed, 10 size warnings, zero errors

Pinned Rust 1.85 lacks its Clippy component locally. Rust 1.85 compilation and
tests passed; Clippy passed on the installed stable toolchain.

## Boundary

This batch adds no live replacement, safety backup, restore journal, rollback,
crash recovery, encryption, custom adapter execution, Tauri, TypeScript,
Svelte, or Poodle dependency.

## Posture

`strict-ready`

Card 007 is complete. Card 008 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start card 008 for journaled restore publication and
crash recovery.
