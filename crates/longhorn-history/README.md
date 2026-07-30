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

Navigation, retention pruning, grouping clocks, encoded weight, paged
projections, persistence encoding, transition journals, host adapters, and UI
belong to later cards.

## Loophole parity and corrections

| Mechanic | Card 062 evidence | Shared result |
| --- | --- | --- |
| typed DAW mutation | represented by a fixture enum only | payload stays generic; Pulse keeps all 83 variants |
| inverse | fixture policy returns a typed inverse | consumer-owned and rejectable |
| adjacent automatic coalesce | rename fixture retains the first id and sequence | retained |
| coalesce to no-op | fixture returns explicit removal | retained with an explicit structural outcome |
| divergent record | imported applied/future shape clears the exact future ids | retained |
| default limit 100 | shared default is 100 | retained as a finite admission bound; oldest-entry pruning waits for Card 064 |
| full undo/redo persistence shape | `LinearHistoryState<P>` round-trips without encoding | structural shape retained; envelope and codec wait for Card 065 |
| current labels and depths | current, next-undo, next-redo, applied, and future accessors | retained as pure state; authoritative pages wait for Card 064 |
| generated ordinal ids | caller injects bounded ids | corrected: no ambient allocation, time, or randomness |
| saturating counters | checked revision and sequence advancement | corrected: overflow fails closed |
| standalone no-op record | donor can retain it | corrected: explicit ignored no-op with no revision or future change |
| incompatible persisted history | donor can silently discard it | prohibited; visible compatibility handling waits for Card 065 |
| gesture grouping | donor API is not live-wired | identity only here; explicit grouping policy waits for Card 064 |
| undo/redo/jump | donor mutates history before fallible apply | deliberately deferred to Card 063 plan/apply/commit |

The donor repository remains unchanged. This table characterizes the audited
Loophole commit recorded in research memo 015.
