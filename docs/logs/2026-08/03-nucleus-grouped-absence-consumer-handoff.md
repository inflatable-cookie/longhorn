# Nucleus Grouped Absence Consumer Handoff

Date: 2026-08-03
Longhorn source: g01.019 Cards 135-137
Nucleus target: contract 032, g05.046
State: Longhorn absence primitive complete; Nucleus restore incomplete

## Available Primitive

Use `BackupAdapterStateEvidence::{Absent, Present}` everywhere the seven
Nucleus grouped adapters describe target or prior state.

`BackupAdapterInspectRequest::source_state()` supplies verified archive truth.
For `Absent`, require zero archive payloads and return an absent target preview.
For `Present`, require the current single file or SQLite payload and return its
semantic digest. Convert observed live presence independently for the preview's
current evidence.

`stage` returns zero target payloads for an absent target and zero rollback
payloads for an absent prior state. `apply` uses
`request.expected_evidence().is_absent()` for deletion. `verify` returns
explicit observed state and can inspect `request.kind()` plus the expected
state. The file and SQLite adapters keep their existing atomic-write and
native backup/restore authorities.

## Exact Nucleus Changes

- update `backup_domains/file.rs` and `backup_domains/sqlite.rs` for the new inspection, stage, apply, and verify signatures
- preserve SQLite main/WAL handling; absent SQLite deletes main, WAL, and SHM through the existing adapter path
- replace grouped execution receipt `.restored()` use with `.entries()` and preserve target/rollback evidence in the durable boot receipt projection
- retain the exact seven-domain selection and single group confirmation
- change `absent_optional_domain_keeps_grouped_restore_unavailable` into a successful prepare/execute fixture that proves deletion
- add target-absence and rollback-to-absence interruption/restart coverage across the Nucleus boot coordinator
- keep restore commands and Settings capability gated until the complete g05.046 lifecycle matrix passes

## Authority Boundary

Longhorn owns state binding, payload-shape validation, confirmation, journal,
apply order, verification, rollback, recovery, and receipts. Nucleus owns
closing live SQLite and every writer, durable restart requests, boot ordering,
domain meaning, and user-facing status.

Do not infer absence from payload count inside orchestration. Do not synthesize
a digest or empty document. Do not clear the pending request until the grouped
receipt or verified recovery is durably projected.

## Resume Gate

Update the pinned Longhorn source, migrate the two adapter implementations and
boot receipt projection, then resume the paused g05.046 fixture. Longhorn's
shared target-absence blocker is closed; Nucleus restore is not yet complete.
