# History Persistence And Committed Transition Stream

Date: 2026-07-30
Card: 065
Roadmap: g01.011

## Result

Added strict structural persistence, independent registered payload codec
compatibility, explicit discard recovery, and one payload-free transition
record for every committed linear mutation.

No Loophole files changed. Product snapshots, model revisions, paths, journal
files, checkpoints, replay, and durability remain consumer authority.

## Structural Envelope

Version 1 uses the fixed `longhorn.linear-history` family and carries:

- independent structural and payload codec versions
- exact history authority and revision
- linear mode
- retention limits and retained-baseline evidence
- monotonic next insertion sequence
- one canonical oldest-to-newest entry list
- explicit current position
- bounded metadata and encoded payload bytes

The golden fixture is
`crates/longhorn-history/fixtures/history/linear-v1.json`.

`HistoryPersistenceLimits` bounds untrusted envelope bytes before JSON parse.
The codec family is a validated lowercase bounded identity. Encoding requires
every entry's admitted weight to equal the exact current codec byte length.

## Compatibility And Recovery

Structural and payload versions migrate separately. Structural hooks receive
raw structural JSON. Registered payload codecs migrate their own opaque bytes.
Both advance exactly one version per step; missing, skipped, future, or
mis-stamped steps reject.

Current load validates:

- strict format and unknown-field policy
- authority, mode, versions, limits, cursor, and baseline
- ids, labels, sequences, revisions, topology, count, and retained weight
- exact source byte length
- current payload decode
- consumer inverse, no-op, and encoded-weight policy

Validation completes before an authority is returned. Corrupt, foreign,
future, unbounded, or incompatible input cannot replace live state.

Successful load records `Preserved` or exact structural/payload `Migrated`
evidence. Rejection returns a typed error. Starting fresh requires the
separate `discard_persisted_history` call and a
`HistoryDiscardRecoveryReceipt`; no parse path creates empty history.

## Committed Transitions

`HistoryCommittedTransition` contains authority, prior revision when one
exists, committed revision, and payload-free kind evidence for:

- record addition
- coalesced replacement or removal
- navigation
- limit change and its exact pruning
- checked import
- explicit persistence discard
- committed reset

Record-driven and limit-driven pruning remain part of the one structural
revision that caused them. Ignored no-ops, unchanged limits, empty resets,
stale requests, failed navigation, codec failure, and policy rejection expose
no transition.

`reset_committed` preserves insertion-sequence monotonicity while clearing
entries and retained baseline after consumer model success.

## Loophole Evidence

The Loophole-shaped adapter:

1. records and coalesces typed Pulse-shaped payloads
2. stores the structural envelope inside a consumer project snapshot
3. keeps product revision outside the envelope
4. appends consumer replay commands paired with committed transitions
5. reloads the checkpoint and replays the valid journal suffix
6. reproduces the exact recorded structural transition
7. retains cross-session undo

An injected journal append failure occurs after a successful in-memory
transition. The history remains committed while the durability failure stays
separate and the failed record does not enter the durable suffix.

The fixture does not claim live Loophole migration. Pulse keeps its mutation
vocabulary, product apply, project format, autosave, journal format,
checkpoint selection, fsync, torn-tail policy, and recovery UI.

## Second Shape

A non-editor typed counter fixture proves:

- exact golden encode and preserved round trip
- authoritative applied/current/future reconstruction
- independent structural and payload migration
- future-version rejection
- corruption, unknown-field, authority, identity, cursor, and byte-bound
  rejection
- explicit discard recovery
- codec-byte and policy-weight agreement

## Boundary Audit

- `longhorn-history` depends only on `longhorn-core`, `serde`, and
  `serde_json`
- no filesystem, environment, config domain, backup, or storage root
- no product snapshot or model revision
- no journal writer, fsync, checkpoint, autosave, or replay choice
- no Tauri, bridge, async runtime, Svelte, or Poodle
- no renderer payload
- no branch persistence

## Validation

- `effigy test:history-core`
- 37 history integration fixtures plus one private corrupt-plan fixture
- `cargo +1.85.0 test -p longhorn-core -p longhorn-history`
- `cargo clippy -p longhorn-core -p longhorn-history --all-targets -- -D warnings`
- `cargo doc -p longhorn-history --no-deps`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-history-persistence`
- `effigy qa:northstar:g01-history-cards`
- `effigy qa:docs`
- `effigy qa:northstar`
- `effigy qa`
- `git diff --check`

## Next

Card 066 is ready. Generate the metadata-only client and compose narrow Tauri,
per-instance Svelte, and public-Poodle edges without exposing product
payloads.
