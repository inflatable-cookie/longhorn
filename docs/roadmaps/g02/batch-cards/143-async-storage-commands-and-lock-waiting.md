# 143 Async Storage Commands And Lock Waiting

Status: complete
Owner: Tom
Roadmap: g02.004 batch 1
Governing refs: contracts 001, 004, and 010; research memo 018
Depends on: none
Auto-start next card: no
Completed: 2026-08-03

## Objective

Take fsync-heavy storage commands and coordination lock waits off the Tauri
main thread without changing the invoke wire surface.

## Scope

- `crates/longhorn-tauri-config/src/commands.rs`
- `crates/longhorn-tauri-settings/src/commands.rs`
- `crates/longhorn-tauri-command/src/commands.rs`
- `crates/longhorn-config/src/coordination.rs` wait loop

## Steps

1. Convert storage-heavy `#[tauri::command]` handlers to `async` and run the
   blocking work through `spawn_blocking`; command names, payloads, and
   receipts unchanged.
2. Keep the file-lock retry loop, but only ever run it on blocking threads;
   document the wait budget at the coordination boundary.
3. Prove main-thread responsiveness under contended-lock and
   storage-transition scenarios in the mock host proofs.
4. Verify consumer-facing invoke fixtures byte-identical.

## Acceptance Criteria

- no storage command executes IO or lock waits on the event thread
- invoke wire surface unchanged; consumer fixtures pass untouched
- config, settings, command suites, and workspace QA pass

## Evidence Required

- responsiveness proof receipts
- unchanged invoke fixture confirmation
- QA receipts

## Stop Conditions

- a consumer composes command handlers in a way that observes handler
  signatures directly (coordinate before landing)
- async conversion forces a public assembly-API break beyond signatures the
  consumers already re-assemble

## Evidence

- 22 commands async over `spawn_blocking`; wire surface unchanged
- coordination wait budget documented at the boundary
- parked-service responsiveness proof
- log: `docs/logs/2026-08/03-host-thread-and-storage-coordination.md`

## Next Task

Promote Card 144.
