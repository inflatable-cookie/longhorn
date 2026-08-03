# Dense Fork-tree Persistence And Migration

Date: 2026-08-03
Card: 072
Roadmap: g01.017

## Result

`longhorn-history-tree` now owns strict bytes-only graph persistence:

- stable `longhorn.history-tree` structural family and version 1 envelope
- independent consumer payload family/version
- RFC 4648 base64 payload strings instead of JSON numeric arrays
- deterministic identity-ordered complete graph encoding
- registered exact next-step structural and payload migrations
- explicit per-instance byte limits under a 1 GiB hard ceiling
- complete validation before any replacement authority returns

The optional tree reuses `HistoryPayloadCodec` and its migration types. The
shared target constructor and consuming migration-step accessor are now public;
the linear persistence behavior and dependency direction do not change.

## Failure And Migration Evidence

The checked matrix covers preserved, structural-only, payload-only, and
combined migration. Missing and skipped steps reject. Foreign format, history,
or payload family; future versions; unknown fields; invalid base64; payload
weight drift; truncated or invalid JSON; and configured oversize all reject.

Topology corruption covers missing parents and branch heads, invalid current
position and preferred children, duplicate sequence and revision, invalid next
sequence, and dangling checkpoint refs. `load` owns no live graph reference and
returns no graph on failure. Current bytes round-trip encode/load/encode exactly
against `fixtures/history/tree-v1.json`.

## Density And Allocation Evidence

The release fixture matches Card 068: 2,112 nodes, 65 branch refs, 256 payload
bytes per node, and 540,672 retained payload bytes.

| Metric | Card 068 numeric arrays | Card 072 base64 |
| --- | ---: | ---: |
| whole envelope | 7,534,856 B | 1,369,417 B |
| encoded payload text | 1,911,872 B | 726,528 B |
| release encode | 7,052 us | 1,581 us |
| release load | 15,436 us | 3,994 us |

The whole envelope is 5.50 times smaller. Timings are local observations, not
acceptance thresholds. Base64 adds one deterministic four-thirds text
expansion per payload. Encode holds one codec byte buffer and one base64 string
transiently per node while building the envelope. Load decodes one owned byte
buffer per node; migration consumes that buffer without cloning it before the
consumer codec creates typed payload authority. No eager alternate-path
projection is serialized.

## Storage Authority Audit

The public API accepts and returns byte slices/vectors only. It contains no
filesystem path, directory convention, file open/write, rename, fsync,
snapshot, journal, autosave, or crash-recovery behavior. Consumers retain all
storage placement and durability policy. Checkpoints serialize only bounded
opaque consumer refs, never checkpoint data.

## Next Task

Execute Card 073. Add bounded metadata protocols and opt-in branch/path clients
while keeping the default projection linear.
