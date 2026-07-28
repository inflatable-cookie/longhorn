# Custom Backup Adapters And Consumer Conformance

Date: 2026-07-28
State: complete implementation batch

## Outcome

- replaced the custom-adapter refusal seam with a schema-opaque adapter
  contract inside `longhorn-config`
- added immutable capture and restore capability declarations
- added coordinated-bounded, external-snapshot, and excluded capture modes
- added excluded, separately receipted, and failure-atomic restore declarations
- added confined multi-payload adapter namespaces, stable ordering, bounds,
  checksums, and capture receipts
- added transaction-authority descriptions to consistency groups
- moved external snapshot work outside the Longhorn store coordinator
- added side-effect-free adapter restore inspection and confirmation binding
- added explicit adapter restore execution with a selectable atomicity floor
- retained custom domains outside the ordinary file restore transaction

## Restore Boundary

The adapter retains product schema and transaction authority. Longhorn passes
verified payloads and sequences declared behavior. It does not interpret a
database, invent an external journal, or treat an adapter as an ordinary
configuration file.

Inspection binds archive digest, domain, adapter id, declared participation,
semantic target evidence, and exact current semantic evidence. Execution
reinspects immediately before mutation. A caller may require failure atomicity;
a separate adapter is then refused before its restore method runs. Verified
target and rollback outcomes must match the confirmed semantic evidence.

Failure-atomic participation is an adapter contract: private staging, exact
current-state preservation, durable journaling, publication verification,
rollback, and rollback verification all remain under the declared external
transaction authority. A weaker adapter receives a visibly separate receipt.

## SQLite Evidence

The test-only SQLite adapter:

- opens a WAL-mode live database
- captures through SQLite's online backup API
- verifies the private snapshot with `PRAGMA quick_check`
- proves the committed WAL row is present in the snapshot
- proves the live WAL bytes are unchanged by capture
- publishes restore through SQLite's restore API
- verifies semantic target evidence after restore
- is refused before mutation when failure atomicity is required

`rusqlite` is an exact test-only dependency. The normal `longhorn-config`
package graph has no SQLite dependency.

## Consumer Evidence

Read-only donor inspection informed three schema-opaque fixtures:

- Loophole machine, window, layout, and Surface-shaped state
- Soundcheck settings and main-window state
- Bovine workspace root and presentation state

Each fixture uses its own domain id, fields, storage class, and validator. All
three complete capture, ZIP encode/inspect, restore planning, private staging,
journaled publication, and exact-byte round trip without product types in
Longhorn. No donor repository was modified.

## Validation

- focused custom adapter and SQLite tests passed
- `cargo clippy -p longhorn-config --all-targets -- -D warnings` passed
- `cargo +1.85.0 test --workspace` passed with 122 Rust tests
- `effigy qa` passed
- `effigy doctor` reported 22 warning-only size findings and zero errors

## Boundary

No SQLite runtime adapter, donor schema, donor write, Tauri, Svelte, Poodle,
settings UI, server synchronization, or consumer migration was added.

## Posture

`strict-ready`

Card 010 is complete. Card 012 is the sole ready lane and is not auto-started.

## Next

Review and explicitly start card 012 for storage-profile transition and legacy
import.
