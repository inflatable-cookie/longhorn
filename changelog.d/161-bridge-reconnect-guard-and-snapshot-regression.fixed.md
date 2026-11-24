A connection that went offline with `retryForbidden` can no longer reconnect with
a retrying class. `BridgeConnectionMachine::reconnect` (Rust) and
`BridgeConnectionRuntime.reconnect` (TypeScript) refuse it as an invalid
transition and leave the connection offline with its reason intact. A `never`
reconnect still goes offline as before.

An older same-session snapshot no longer replaces a newer accepted baseline. The
stream tracker refuses it as `staleSnapshot`, whenever it resolves, and keeps the
baseline. A stale resync reply is dropped: the TypeScript stream keeps its current
snapshot and stays live, where it previously moved the baseline back and could
apply an already-applied event a second time.

The connection state-to-state transition table is now declared once,
`BridgeConnectionState::ADMITTED_TRANSITIONS`, and generated as
`BRIDGE_ADMITTED_CONNECTION_TRANSITIONS`. The TypeScript runtime reads that
table in place of its hand-written copy.

The transition and service receipts expose the fields they put on the wire:
`at()` and `previous()` on the connection receipt, and `generation()`,
`ownership()`, `action()` and `previous()` on the service receipt.

## Breaking

- `BridgeSnapshotDecision` gains `StaleSnapshot`, which is `staleSnapshot` on the
  wire and in `BRIDGE_SNAPSHOT_DECISIONS`. A Rust `match` with no wildcard arm
  must handle it. Migration: treat it like `SupersededSession`, as a refused
  snapshot that keeps the current baseline.
- The TypeScript `BridgeSnapshotDecision` type gains `"staleSnapshot"`. Migration:
  handle it beside `"supersededSession"` and `"wrongDomain"`. The stream does not
  throw on it.
