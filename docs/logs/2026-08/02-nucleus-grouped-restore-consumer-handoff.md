# Nucleus Grouped Restore Consumer Handoff

Date: 2026-08-02
Longhorn source: g01.019 Cards 128-131
Nucleus target: contract 032, g05.046
State: Longhorn primitive complete; Nucleus restore incomplete

## Available Primitive

Nucleus can register each selected custom domain with
`BackupAdapterRestoreParticipation::GroupedFailureAtomic` and implement the
generic `BackupAdapterGroupedRestore` stage, apply, and verify boundary.

The host flow is:

1. inspect one verified archive with the complete adapter catalogue
2. call `plan_grouped_adapter_restore` with the exact selected seven-domain set
3. present and retain the single group confirmation digest
4. quiesce all Nucleus authorities and close live SQLite connections
5. call `execute_grouped_adapter_restore` offline
6. schedule restart only from the exact terminal receipt
7. during boot, before opening authorities, call
   `recover_grouped_adapter_restore` with the exact catalogue

Longhorn owns complete staging, payload bounds, durable journalling, stable
apply order, reverse rollback, semantic verification, recovery state, and
receipts. The API is Rust-only and does not require renderer state.

## Nucleus Work Still Required

- freeze the exact seven-domain selection and adapter registration
- implement opaque stages for each domain, using native SQLite snapshot/restore
  rather than main/WAL file copying
- stop windows, background tasks, services, database pools, and every external
  writer before execution or recovery
- keep those authorities closed until the receipt is terminal
- map terminal receipts to restart, recovery UI, and operator diagnostics
- prove app restart, interrupted boot recovery, and capability policy in Nucleus

Do not call the seven adapters sequentially. Do not expose restore as complete
until Nucleus proves app-wide quiescence and restart scheduling. Longhorn does
not supply those product policies.

## Resume Gate

Resume Nucleus g05.046 against
`docs/reference/grouped-adapter-restore.md` and contract 004. The Longhorn
library blocker is closed; the remaining blocker is Nucleus-owned lifecycle
orchestration and conformance.
