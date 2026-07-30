# longhorn-history

Pure typed structural history. Consumers own payload meaning, product apply,
rollback, labels, persistence codecs, storage, journals, and recovery.

Card 062 supplies:

- bounded history, entry, kind, group, and plan identities
- distinct history revision and entry insertion sequence
- generic typed entries
- injected inverse, no-op, and adjacent-coalescing policy
- validated applied/future linear state
- record-after-product-success with exact future clearing
- explicit add, replace, remove, and ignored-no-op outcomes

Card 063 adds:

- immutable undo, redo, and entry-id checkout plans
- bounded inverse and forward payload batches
- exact source and target position metadata
- injected atomic consumer transactions
- exclusive revalidation, apply, then structural commit
- exact rolled-back and terminal rollback-failed outcomes
- stale, duplicate, oversized, foreign, and corrupt-plan rejection
- authoritative navigation receipts

Card 064 adds:

- adjacent and active-group coalescing contexts
- explicit open, close, cancel, teardown, and committed-navigation boundaries
- timed groups using injected monotonic milliseconds and consumer duration
- consumer-measured encoded payload weight
- count and weight pruning with durable retained-baseline evidence
- checked runtime limit changes and safe future-tail truncation
- payload-free summaries and bounded authoritative metadata pages

Persistence encoding, transition journals, host adapters, and UI belong to
later cards.

## Loophole parity and corrections

| Mechanic | Shared evidence | Shared result |
| --- | --- | --- |
| typed DAW mutation | represented by a fixture enum only | payload stays generic; Pulse keeps all 83 variants |
| inverse | fixture policy returns a typed inverse | consumer-owned and rejectable |
| adjacent automatic coalesce | rename fixture retains the first id and sequence | retained |
| coalesce to no-op | fixture returns explicit removal | retained with an explicit structural outcome |
| divergent record | imported applied/future shape clears the exact future ids | retained |
| default limit 100 | shared default is 100 | retained with exact oldest-applied pruning and baseline evidence |
| full undo/redo persistence shape | `LinearHistoryState<P>` round-trips without encoding | structural shape retained; envelope and codec wait for Card 065 |
| current labels and depths | current, next-undo, next-redo, applied, and future metadata | retained through authoritative payload-free summaries and pages |
| generated ordinal ids | caller injects bounded ids | corrected: no ambient allocation, time, or randomness |
| saturating counters | checked revision and sequence advancement | corrected: overflow fails closed |
| standalone no-op record | donor can retain it | corrected: explicit ignored no-op with no revision or future change |
| incompatible persisted history | donor can silently discard it | prohibited; visible compatibility handling waits for Card 065 |
| gesture grouping | donor API is not live-wired | explicit or timed; Loophole may inject 750 ms and construct product-owned compounds |
| undo/redo/jump | typed undo, redo, and entry-id checkout fixtures | corrected: plan is immutable; injected apply is atomic; history commits only after success |

The donor repository remains unchanged. This table characterizes the audited
Loophole commit recorded in research memo 015.

## Navigation transaction

`plan_navigation` never changes history. `execute_navigation` revalidates the
plan before calling the injected product transaction. It holds exclusive
history authority through product apply and structural commit, so history
cannot advance in the gap between them.

The consumer transaction returns success, exact rollback, or rollback failure.
Only success moves history. Rollback failure is terminal product evidence; it
does not claim model restoration or history success. Product apply remains
consumer code.

## Grouping and retention

Ordinary adjacent coalescing and active-group coalescing are distinct policy
contexts. A continuing group must merge or remove into one atomic
consumer-owned payload. Longhorn never constructs a product compound.

Explicit groups require matching caller-injected ids. Timed groups use an
opaque consumer key, candidate group id, injected monotonic milliseconds, and
consumer duration. No clock or fixed gesture duration exists in the crate.
Open groups are transient and never enter `LinearHistoryState`.

Every retained entry carries consumer-measured encoded payload weight. Record
and limit changes prune the oldest applied prefix first and advance
`HistoryRetainedBaseline`. A tighter limit may then discard the farthest
future tail without breaking the next-redo chain. Receipts list both classes
separately.

## Projections

`project_summary` reports revision, mode, applied/future depths, current id,
next labels, retained weight, and baseline evidence. `project_page` returns
bounded payload-free entries newest-first across authoritative future,
current, and past positions. Renderer memory is not required to reconstruct
redo.
