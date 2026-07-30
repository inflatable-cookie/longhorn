# History Coalescing, Grouping, Retention, And Projections

Date: 2026-07-30
Card: 064
Roadmap: g01.011

## Result

Completed the pure public linear mechanics in `longhorn-history`: contextual
coalescing, explicit and injected-time groups, count and encoded-weight
retention, durable baseline evidence, checked limit changes, and bounded
payload-free pages.

No Loophole files changed.

## Coalescing And Grouping

Consumer policy now receives one explicit coalescing context:

| Context | Keep separate | Replace | Remove |
| --- | --- | --- | --- |
| ordinary adjacent | add a new entry | retain prior id and sequence | remove prior entry |
| continuing group | reject: not one atomic payload | retain one grouped payload | remove grouped payload |

The continuing-group rule keeps one gesture as one undo step without teaching
Longhorn how to construct a product compound. Loophole's fixture constructs
and reverses its own compound.

Explicit groups use caller-injected group ids and exact open, append, close,
cancel, and teardown calls. Ordinary records cannot bypass an open group.
Timed groups use:

- caller-injected candidate group id
- opaque consumer group key
- injected monotonic milliseconds
- consumer-selected nonzero duration

Same key and duration continue while elapsed time is below the duration.
Exact expiry closes the old group as `TimedOut`. Key or duration changes close
it as `Replaced`. Time regression rejects without changing history or the
active group.

Open groups are transient. Structural import, authority replacement, committed
navigation, explicit close/cancel, limit changes, and teardown leave no active
group. Failed navigation retains exact structural and transient history.

## Retention

Each entry stores the exact encoded payload weight reported by consumer
policy. Longhorn does not choose or run a payload codec in this card.

Record admission and limit changes enforce both:

- retained entry count
- total retained encoded weight

Recording prunes the oldest applied prefix until the new current entry fits.
`HistoryRetainedBaseline` tracks cumulative count and weight plus the last
pruned id and sequence. The record receipt lists every entry absorbed into the
baseline.

Limit changes use the same applied-prefix rule. If the new budget still does
not fit after the applied path is empty, they discard the farthest future tail
first. This preserves the current product state and next-redo chain. Baseline
advancement and future truncation are separate receipt fields.

Zero budgets, defensive-ceiling violations, oversized single payloads,
retained-weight overflow, baseline overflow, stale revisions, and incompatible
label limits fail without mutation.

## Projections

`project_summary` returns:

- history id, revision, and linear mode
- applied and future depths
- current entry id
- next undo and redo labels
- retained entry count and encoded weight
- retained-baseline evidence

`project_page` returns bounded payload-free entries newest-first:

1. farthest future through next redo
2. current
3. newest past through oldest retained past

Every entry carries authoritative `Future`, `Current`, or `Past` position.
Pages include exact offset, total, both truncation flags, revision, and
baseline evidence. Aura-style remembered redo is no longer required.

## Loophole Evidence

The Loophole-shaped fixture proves:

- default entry limit remains 100
- entry 101 prunes entry 1 exactly
- 750 ms is selectable by Loophole, not fixed by Longhorn
- 749 ms continues a timed group
- 750 ms opens a new group
- incompatible grouped mutations become one Loophole-owned compound
- compound inverse order remains consumer-owned
- navigation and explicit closure stop later coalescing

The donor grouping API still has no claimed live call site. This card preserves
the capability without claiming migration or wiring.

## Second Shape

A weighted non-editor document fixture proves count and weight pruning,
baseline overflow, safe future truncation, stale limit failure, and exact
receipts. A separate projection fixture proves complete past/current/future
pagination without Tauri, Svelte, Poodle, config, bridge, journal, or renderer
state.

## Boundary Audit

- no ambient clock or fixed duration
- no product group enum or compound constructor
- no payload codec implementation
- no product model, revision, snapshot, or journal
- no persistence encoding
- no generic renderer payload
- no Tauri, async runtime, Svelte, Poodle, config, or bridge dependency
- no branch API

## Validation

- `effigy test:history-core`
- 29 history integration fixtures plus one private corrupt-plan fixture
- `cargo +1.85.0 test -p longhorn-core -p longhorn-history`
- `cargo clippy -p longhorn-core -p longhorn-history --all-targets -- -D warnings`
- `cargo doc -p longhorn-history --no-deps`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-history-mechanics`
- `effigy qa:northstar:g01-history-cards`
- `effigy qa:docs`
- `effigy qa:northstar`
- `git diff --check`

## Next

Card 065 is ready. Add structural and payload persistence compatibility,
visible recovery, and committed transition records without taking ownership
of product snapshots, storage paths, or journal I/O.
