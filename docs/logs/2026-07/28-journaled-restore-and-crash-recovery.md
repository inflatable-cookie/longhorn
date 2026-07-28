# Journaled Restore And Crash Recovery

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added durable live restore execution over the card 007 staging set
- added verified pre-restore safety publication and journal-backed retention pin
- added bounded exact rollback payloads, phased recovery, and write blocking
- added typed restore load states and coordinated multi-domain load-sets
- routed destructive schema rewrite through verified pre-migration backup

## Transaction

Execution reacquires the store-wide coordinator, recovers any older operation,
and rereads exact staged current evidence. Rollback capture uses the caller's
backup memory ceilings. The selected-domain safety snapshot is encoded,
published durably, reopened, hashed, and inspected before live mutation.

The authority-local journal records only registered domain id, storage class,
relative path, action, exact old and target evidence, rollback payload name,
plan digest, phase, and safety archive path and digest. Recovery resolves every
path again through the live registry and injected roots. Archived or ambient
absolute target paths never become write authority.

Each changed domain uses the existing single-file atomic publisher. Deletes
sync their registered parent. Full-set target verification precedes durable
success. This is failure-atomic terminal behavior, not portable cross-file
atomic visibility.

## Failure And Recovery

Every post-journal ordinary failure invokes complete old-set rollback under the
same guard. Return states are exact:

- `NoLiveMutation`
- `RolledBack`
- `RecoveryRequired`

Recovery rolls back `prepared`, `applying`, `verifying`, `rolling-back`, and
`recovery-required` journals. It cleans verified `succeeded` and `rolled-back`
terminals. A corrupt or missing rollback payload retains the journal, exposes
typed recovery-required loads, and makes later mutation fail before patch
code. Repeated successful recovery is a no-op.

The journal digest pins the safety archive until terminal cleanup. Callers can
feed `ConfigStore::restore_safety_pin` into retention planning.

## Reads And Migration

Lock-free loads return distinct active-restore and recovery-required
unavailable states. `with_coordinated_load_set` recovers first, then holds the
store coordinator across all member reads. It is the explicit cross-domain
generation boundary.

`rewrite_migrated_domain` accepts only an in-memory migrated older source. It
requires `pre-migration` metadata, captures exact old bytes, publishes and
verifies the safety archive, and uses the same journal, publisher, verification,
rollback, and recovery path as restore.

## Evidence

- successful restore verifies live bytes and exact safety-backup contents
- every nonterminal journal phase rolls back exact old bytes
- recovery is idempotent
- corrupt rollback retains the marker and blocks loads and mutation
- injected post-journal failure rolls back before return
- coordinated load-set excludes a competing writer across member reads
- destructive migration reaches current schema and preserves exact old source
- `longhorn-config`: 89 tests passed
- Rust 1.85 workspace: 94 tests passed
- Rust 1.85 rustdoc and stable Clippy passed with warnings denied
- Effigy QA passed; Doctor reports 15 size warnings and zero errors

## Boundary

No encryption, custom adapter execution, Tauri lifecycle, settings UI,
TypeScript, Svelte, or Poodle dependency was added. Poodle remains visual
authority.

## Posture

`strict-ready`

Card 008 is complete. Card 009 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start card 009 for the optional age v1 archive adapter.
