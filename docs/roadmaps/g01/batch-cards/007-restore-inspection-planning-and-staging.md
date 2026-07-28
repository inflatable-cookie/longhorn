# 007 Restore Inspection, Planning, And Staging

Status: planned after card 006
Owner: Tom
Roadmap: g01.002 batch 3
Governing refs: contracts 001, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Turn a verified plaintext archive into a non-mutating compatibility report,
confirmation-bound restore plan, and complete current-schema staging set.

## Scope

- app and producer compatibility checks
- registered, unknown, absent, source-preserved, excluded, and unavailable
  domain reporting
- source-to-target schema and migration-path inspection
- explicit restore scope, skip, and conflict decisions
- create, replace, delete, migrate, and unchanged action model
- current present/absent, length, and SHA-256 evidence
- canonical plan digest bound to archive hash and current evidence
- stale-plan recheck under the store coordinator
- side-effect-free in-memory migration, encode, and validation
- complete private target staging before live mutation
- machine-readable inspection and staging receipts

## Public Behavior

Inspection performs no store mutation, migration rewrite, safety backup, or
retention change. Unknown app identity blocks generic restore. Unknown domains
are reported and skipped only by explicit plan scope.

The plan binds the exact archive instance, current domain evidence, selected
actions, and conflict choices. Confirmation is valid only for that plan.
Execution preparation reacquires the coordinator, rereads current evidence,
and rejects stale state before returning a staged transaction input.

Migrations run against archive payloads in memory. Staged output uses the
current registered envelope and passes codec validation. Future, corrupt
source-preserved, missing migration, unavailable adapter, and failed
validation block any selected domain.

## Out Of Scope

- publication to live domain paths
- safety backup, restore journal, rollback, or crash recovery
- encrypted archive decryption
- custom adapter execution
- settings UI or Tauri confirmation dialog

## Acceptance Criteria

- inspection is byte-for-byte non-mutating
- reports every manifest domain and exclusion exactly once
- app mismatch, unknown domain, future schema, corrupt source evidence, and
  missing migration are distinct
- explicit subset restore cannot silently add or omit a domain
- action diff handles present/absent create and delete
- confirmation digest changes with archive, action, conflict, or current
  evidence
- mutation between preview and recheck produces stale-plan failure
- every selected migratable payload reaches current schema and validates
- one failed selected domain prevents the complete staging set
- unchanged encoded evidence avoids a later publication action

## Stop Conditions

- inspection writes a migration, backup, retention record, or live domain
- a selected incompatible domain can be silently skipped
- confirmation is not bound to current evidence
- migration requires mutating the source archive
- the card expands into journaled publication

## Next Task

Run after card 006 closes. Then activate journaled restore and crash recovery.
