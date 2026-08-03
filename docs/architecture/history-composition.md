# History Composition

Status: promoted  
Owner: Tom  
Updated: 2026-08-03
Contracts: `../contracts/003-error-concurrency-and-recovery.md`,
`../contracts/008-history-state-and-versioning.md`,
`../contracts/010-rust-typescript-ipc-and-events.md`,
`../contracts/012-distribution-and-compatibility.md`, and
`../contracts/013-svelte-and-poodle-adapter-lifecycle.md`

## Selection Rule

Start with typed linear history. Add persistence, a product journal, Tauri,
Svelte, and Poodle only where the app needs them.

| Need | Rust | TypeScript | Consumer authority |
| --- | --- | --- | --- |
| Record and navigate | `longhorn-history` | `@longhorn/history` | payload policy and product apply |
| Structural reload | same crate | same root | codec, storage, migration choice |
| Durable product recovery | same crate | none | product snapshot and journal |
| Tauri metadata host | add `longhorn-tauri-history` | add `/tauri` | caller authorization |
| Per-window reactive state | same Rust graph | add `/svelte` | instance lifetime |
| Linear history panel | same Rust graph | add `/poodle` | placement and product copy |

The pure Rust crate depends only on `longhorn-core`, Serde, and JSON. The
framework-neutral TypeScript root depends only on `@longhorn/core`. Svelte and
Poodle are optional peers. Config, bridge, Tauri, journal, Svelte, and Poodle
do not enter the minimal graph.

## Authority Split

Longhorn owns:

- bounded linear structural state
- applied and retained-future topology
- record, coalesce, group, retention, and pruning mechanics
- immutable revision-bound navigation plans
- commit only after consumer apply succeeds
- strict structural envelopes and payload codec version headers
- payload-free summaries, pages, receipts, and invalidation hints

The consumer owns:

- payload variants and inverse meaning
- no-op, coalesce, grouping, and encoded-weight policy
- the canonical product model
- atomic apply and verified rollback
- payload codec bytes and migration
- filesystem path, snapshot, journal, fsync, autosave, and recovery policy
- caller authorization
- project versions, collaboration, and product checkpoints

No renderer message contains product payload. A Tauri permission makes a
handler reachable; it does not grant product navigation authority.

## Record Path

Apply the product mutation first. Call `record_applied` only after product
success.

```text
product validate → product apply → history record/coalesce → committed receipt
                                                        └→ consumer durability
```

The history revision supplied to `AppliedHistoryRecord` must match the live
history. Record failure after product apply is a consumer transaction failure;
the consumer must restore or enter explicit recovery. Longhorn does not
silently repair product state.

Use explicit groups for product transactions. Use timed groups only with
consumer-injected monotonic time and duration. Continuing a group requires one
atomic payload. Close groups at gesture completion, navigation, teardown, and
restore boundaries.

## Navigation Path

Undo, redo, and checkout share one protocol:

1. Plan against exact history revision and stable entry IDs.
2. Apply every typed step through one consumer transaction.
3. Commit structural movement only after apply succeeds.
4. Publish the committed receipt to consumer durability.
5. Treat renderer events as refresh hints.

An apply failure with verified rollback leaves model and history exact. A
rollback failure leaves history exact but cannot claim the model was restored.
Treat it as terminal partial-model evidence and enter product recovery.

Stale and duplicate plans reject before product apply. Checkout uses stable
entry IDs, never list positions.

## Persistence And Recovery

`HistoryPersistence` stores the complete retained linear shape. It does not
store the product.

A Loophole-shaped durable arrangement is:

```text
product snapshot
├─ product revision
├─ canonical product state
└─ encoded Longhorn structural history

product journal suffix
├─ product revision
├─ consumer payload
└─ expected committed history transition
```

Recovery loads the product snapshot and structural history, then replays only
the durable journal suffix. Each replayed history result must equal the
recorded transition. Cross-session undo is available after replay.

An in-memory history commit followed by journal append failure is still an
in-memory commit. Do not report the original product action as uncommitted or
blindly retry it. Record the durable revision gap and use product recovery.

Future structural versions, future codec versions, foreign history identity,
foreign codec family, corrupt payload bytes, invalid topology, and oversized
envelopes fail visibly. Replace live history only after full decode,
migration, policy measurement, and structural validation. Discard requires an
explicit receipt and reason.

Storage location, backup, and recovery use the configuration system's selected
profile. History does not choose paths.

## Renderer And Poodle

The renderer reads a snapshot plus bounded newest-first pages. Each entry has
an authoritative `past`, `current`, or `future` position. Redo state is not
remembered in the renderer.

Install event listeners before the first snapshot. Refresh on newer revisions,
history changes, or authority-epoch changes. Ignore old hints. Events are
non-durable and may be missed.

Create one `HistorySession` per mounted window or panel. Stop or dispose it on
unmount. `@longhorn/history/poodle` uses public controlled Poodle primitives.
Poodle owns visuals. The consumer owns panel placement, product labels, icons,
filters beyond the shared metadata fields, and surrounding shell.

## Proven Shapes

| Shape | Rust graph | TypeScript graph | Selected edges |
| --- | --- | --- | --- |
| Minimal non-editor | core, history | core, history root | direct checked client |
| Loophole-shaped | core, history, Tauri history | core, history root plus `/tauri`, `/svelte`, `/poodle` | typed Pulse-like policy, transaction, codec, snapshot, journal, caller authority, Poodle panel |

Both shapes consume produced artifacts. Rust emits the renderer fixtures. The
isolated TypeScript installs reproduce the same public trace. This is
conformance evidence, not a donor cutover.

## Loophole Migration

Consumer migration belongs to g01.015. Do not replace the current history and
journal in one unmeasured edit.

1. Freeze the live mutation, inverse, coalesce, group, limit, apply, snapshot,
   journal, recovery, and panel inventory.
2. Map all current payload variants into a Loophole-owned type implementing
   `HistoryPolicy`. Keep Pulse apply and rollback in Loophole.
3. Define an explicit versioned codec for existing retained history. Reject
   unsupported sources; do not fall back silently to empty history.
4. Run the shared kernel beside the current implementation against copied
   projects and disposable journals. Compare record, navigation, pruning,
   restart, recovery, and cross-session undo traces.
5. Select one history authority at bootstrap. Never dual-write two durable
   histories without one atomic product transaction.
6. Migrate snapshot plus journal state under a staged, receipted operation.
   Keep the old durable source until new publication and restart validation
   succeed.
7. Move metadata reads to the generated protocol. Prove future entries,
   listener gaps, stale commands, and teardown.
8. Mount the Poodle panel only after semantic parity. Keep app-specific rows,
   project copy, and actions in Loophole.
9. Narrow Tauri capabilities. Recheck product authorization on every
   navigation.
10. Remove the old authority only after restart, corrupt-source, journal-gap,
    rollback, and recovery acceptance passes.

Migration rollback reselects the old durable source. It does not translate a
partially committed new runtime back in place.

## Retained, Improved, Rejected, Deferred

| Class | Result |
| --- | --- |
| retained | typed product mutations, inverse/coalesce policy, 750 ms grouping capability, 100-entry default, undo/redo/checkout, product snapshot, journal suffix, recovery, cross-session undo, product labels |
| improved | plan/apply/commit admission, verified failure invariance, exact rollback-failure evidence, strict envelopes, authoritative future pages, listener-first refresh, per-instance teardown |
| rejected | renderer-owned redo, move-before-apply, silent empty fallback, generic payload transport, Tauri capability as product authority, durable-event claim |
| deferred | donor cutover, tree persistence/clients/artifacts, project versions, collaboration, registry publication |

Card 068 proves fork topology privately. Card 069 promotes its semantics while
retaining the prototype as research. Card 070 implements the optional pure
Rust identity, topology, branch, and divergent-record foundation. Linear mode
remains the only compatibility-proved artifact until Card 074. Card 071 adds
atomic mixed-route navigation, protected pruning, and opaque checkpoint replay
accounting without changing Loophole or the linear packages.

## Proof

Run:

```sh
effigy proof:history-system-artifacts
```

Sources live in `examples/history-system-proof/`. Checkpoint evidence is in
`../logs/2026-07/30-linear-history-artifact-proof-and-checkpoint.md`.
