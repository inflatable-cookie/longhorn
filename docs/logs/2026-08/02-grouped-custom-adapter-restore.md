# Grouped Custom-adapter Restore

Date: 2026-08-02
Roadmap: g01.019
Cards: 128-131
State: complete; Card 070 ready

## Result

`longhorn-config` now executes an exact selected set of custom restore adapters
as one failure-atomic transaction. One plan and confirmation bind the verified
archive, sorted domains, adapter identities, per-domain confirmations, target
evidence, and current evidence.

Every adapter stages opaque target and exact rollback payloads without live
mutation. Longhorn validates and syncs the complete payload set and group
journal before applying the first target. It verifies the complete new
generation, or unwinds adapters in reverse order and verifies the complete old
generation. Failure receipts distinguish no mutation, verified rollback, and
recovery required.

## Recovery

The private journal survives interruption during target apply, target
verification, and rollback. Normal loads, mutations, coordinated load sets,
ordinary recovery, and separate adapter execution fail closed while it exists.
Boot recovery requires the exact registered descriptors and grouped adapter
catalogue. Missing or changed adapters and corrupt journals retain the blocking
state.

Recovery needs no renderer or open product authority. The consumer must keep
external databases and services quiescent until recovery completes.

## Compatibility Evidence

- empty, duplicate, separate, unknown, stale, and wrongly confirmed groups fail before mutation
- stage, apply, and verify failures preserve one exact terminal generation
- interruption in applying, verifying, and rolling back recovers the complete old group
- opaque-file and WAL-mode SQLite adapters commit and roll back together
- existing ordinary restore and separately receipted adapters remain green
- generated configuration bindings include the new participation value without renderer execution authority
- Rust 1.85, Clippy, package inventory, focused config, and Northstar path gates pass
- full workspace Rust Clippy and test suites pass

No Nucleus source was edited. No crate, npm package, tag, or hosted release was
published.

## Next

Longhorn returns to Card 070. Nucleus may resume g05.046 from the separate
consumer handoff, but its restore is not complete.
