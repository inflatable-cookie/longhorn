# 142 Client Binding Races And Ordered Events

Status: complete
Owner: Tom
Roadmap: g02.003 batch 2
Governing refs: contracts 001, 010, and 011; research memo 018
Depends on: Card 141
Auto-start next card: no
Completed: 2026-08-03

## Objective

Close the snapshot/destroy client-binding leak and make client-changed
events observe epoch order.

## Scope

- `crates/longhorn-tauri-transfer/src/handler.rs` snapshot and destroy paths
- `crates/longhorn-tauri-transfer/src/commands.rs` event emission

## Steps

1. Bind client epochs under the state lock with a window-liveness recheck so
   a destroy racing snapshot cannot re-insert a binding; assert destroyed
   windows hold no slot.
2. Order `TRANSFER_CLIENT_CHANGED_EVENT` against epoch advancement (emit
   under the lock or sequence emission) so a renderer never retains an older
   epoch after a newer event.
3. On emit failure, surface that the epoch already advanced (typed evidence,
   not a bare string error).
4. Regression: repeated snapshot/destroy cycles never exhaust
   `maximum_client_windows`.

## Acceptance Criteria

- no binding survives its window; capacity stable across cycles
- concurrent snapshots deliver monotonically ordered client events
- transfer suites and workspace QA pass

## Evidence Required

- race regression receipts
- ordering test receipts
- QA receipts

## Stop Conditions

- ordering requires holding the state lock across a Tauri emit that can
  re-enter the handler (would need a queue design decision)

## Evidence

- post-bind liveness recheck undoes race-lost bindings; 32-cycle regression
  against capacity 4 never exhausts client windows
- snapshot/emit serialized behind a state mutex; emit failure no longer
  masks advanced authority (invoke result authoritative, event advisory)
- log: `docs/logs/2026-08/03-transfer-session-truthfulness.md`

## Next Task

Promote Card 143 (g02.004).
