# 182 Host-Supplied `recorded_at`

Status: ready
Owner: Tom
Roadmap: g02.016 batch 2
Governing refs: contract 011; contract 012
Depends on: Card 181 (complete)
Blocks: version captions showing a time in Loophole's HistoryCentre
Auto-start next card: no

## Why

`HistoryEntryMetadata` is label, kind and group. There is no time field
anywhere in either history domain, deliberately: the crates own no clocks, and
a history that invented timestamps would be asserting something it never
observed.

Display needs one anyway. "2m ago" on a version caption is a real requirement
from Loophole's field use, and the host does have a clock.

So the field is **supplied, never derived**. A consumer that knows the time
stamps it at record time; one that does not leaves it absent. Nothing in either
crate reads it — ordering stays structural, and no comparison, sort or
retention rule may consult it.

## Operator Decision — settled 2026-08-12

Entries written before this lands read back as `recorded_at: None`. They simply
have no time, and their captions show none, permanently. Nothing migrates,
nothing backfills, and no envelope fails to load.

The rejected alternative was backfilling a plausible time at read. That would
put an invented observation into stored data, which is worse than an absent one
and impossible to distinguish later.

## Scope

Both domains. `HistoryEntryMetadata` lives in `longhorn-history`, so the
linear domain carries the field as well as the fork tree — the type is shared
and splitting it to avoid the reach would cost more than it saves.

Five surfaces per domain, all following the path `kind_id` already takes:

**`longhorn-history`**
- `src/entry.rs` — the metadata type
- `src/persistence/wire.rs:148`, `encode.rs:51`, `load.rs:274` — the envelope
- `src/projection.rs:33` — the projected record
- `src/protocol/snapshot.rs:128` — the wire record

**`longhorn-history-tree`**
- `src/persistence/wire.rs:47`, `service.rs:84`, `decode.rs:82` — the envelope
- `src/projection/types.rs:144`, `project.rs:166` — the projection
- `src/protocol/path.rs:22` — `ForkEntryRecord`

## Steps

- [ ] Add `recorded_at: Option<HistoryRecordedAt>` to `HistoryEntryMetadata`
      with an accessor, where the newtype wraps epoch milliseconds. A named
      type rather than a bare `u64`, so the unit is in the signature and a
      sequence number cannot be passed by mistake.
- [ ] Keep `HistoryEntryMetadata::new` at three arguments and add
      `with_recorded_at`. A fourth positional `Option` next to two others reads
      as `new(label, None, None, None)` at every call site, and there are seven.
      This is a builder for an optional field, not a compatibility shim.
- [ ] Carry it inert through both envelopes. Absent in stored data decodes to
      `None`; present round-trips unchanged.
- [ ] Carry it to `ForkEntryRecord` and the linear entry record, then
      regenerate bindings.
- [ ] Do not let the tree read it. No ordering, retention or navigation rule
      may consult the field.

## Acceptance

- [ ] `effigy qa` passes, including `check:bindings`.
- [ ] A persistence test proves an envelope written without the field loads as
      `None` rather than failing — the operator decision above, enforced.
- [ ] A persistence test proves a supplied value round-trips through encode and
      decode unchanged.
- [ ] A projection test proves the value reaches `ForkEntryRecord`.
- [ ] Nothing in either crate reads `recorded_at` outside a projection: a grep
      of the source shows reads only where the record is built.

## Evidence

- [ ] The tests above, named in the batch log.

## Stop Conditions

- Stop if the envelope cannot express an absent field without a schema version
  bump. A version bump is a stored-data migration and a separate decision from
  the one already taken.

## Continuation

Card 183 for the topological projection. Poodle has built HistoryCentre v2, so
its stitcher is now readable and that card's planning gap is closed.
