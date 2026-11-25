`BridgeSnapshotDecision` gains `StaleSnapshot`, which is `staleSnapshot` on the
wire and in `BRIDGE_SNAPSHOT_DECISIONS`. A Rust `match` with no wildcard arm
must handle it. Migration: treat it like `SupersededSession`, as a refused
snapshot that keeps the current baseline.
