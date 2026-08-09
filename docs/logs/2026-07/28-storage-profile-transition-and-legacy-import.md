# Storage Profile Transition And Legacy Import

Date: 2026-07-28
State: complete implementation batch

## Outcome

- added a fixed native bootstrap locator derived only from canonical
  application identity
- added missing-default, explicit host-bypass, selected, and typed recovery
  bootstrap states
- added side-effect-free source/target inventory with bounded evidence,
  visible lifecycle policy, overlap checks, and destination conflicts
- added declarative read-only legacy candidates and donor-shaped discovery
  fixtures for Loophole, Soundcheck, Nucleus, and Split-shell
- added confirmation-bound plans, deterministic dual-store coordination,
  staged ordinary publication, durable journals, and locator-last commit
- added schema-opaque adapter transition participation under an external
  quiescence guard
- added verified crash recovery before and after locator commit
- added retained-source receipts and idempotent receipt-bound cleanup that
  cannot include unknown files

## Authority

The locator lives below the fixed native canonical-id bootstrap root. Optional
stable storage names and selected profiles cannot move it. Missing locator
selects the compiled native default. Invalid, corrupt, future, mismatched, or
unknown-profile documents expose recovery and never choose a fallback.

Transition preview reports ordinary, external, cache, log, runtime, secret,
custom-adapter, unknown-file, and conflict state without mutation. Execution
rechecks the complete evidence cut under deterministic authority guards,
publishes and verifies the target, then commits the locator. Source data stays
in place until a separate cleanup plan derived from the committed receipt is
applied.

## SQLite Evidence

The WAL-mode fixture acquires an adapter-owned transition guard, captures and
restores through SQLite native APIs, and verifies semantic target state. The
main database, WAL, SHM, and adapter authority marker never enter ordinary
copy or unknown-file handling. The live source WAL remains unchanged.

## Recovery And Cleanup

Injected failures on both sides of locator commit recover to the one authority
selected by the locator. Cleanup rechecks the committed transition id and
layout digest, exact target bytes, exact retained source bytes, and registered
source-to-target mapping under both store locks. Repeating cleanup reports
already-absent registered files. Unregistered source files remain untouched.

## Validation

- 16 storage-layout integration tests passed
- 36 `longhorn-config` unit tests and 61 domain-store tests passed
- Rust 1.85 workspace tests passed with 130 tests
- stable Clippy passed with warnings denied
- Effigy QA passed
- Effigy Doctor reported 27 warning-only size findings and zero errors
- the normal `longhorn-config` graph retained no SQLite dependency

## Boundary

No donor repository, product schema, SQLite runtime adapter, Tauri runtime,
settings UI, TypeScript, Svelte, Poodle, remote synchronization, or automatic
legacy-root search was added.

## Posture

`strict-ready`

Card 012 and `g01.002` are complete. `g01.003` is at card compilation.

## Next

Compile bounded display geometry, inventory, and pure window-planning cards.
Do not start implementation from the milestone summary.
