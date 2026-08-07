# 150 Cross-channel Store Compatibility Proof And Classification

Status: ready
Owner: Tom
Roadmap: g02.009 batch 1
Governing refs: contracts 018 and 004; research memo 019
Depends on: none
Auto-start next card: no

## Objective

Prove that every persistent store already refuses a store written by a newer
schema without modifying it, and give that refusal one shared classification
so the update surface can explain a channel rejoin.

## Correction To The Original Premise

This card was compiled on the claim that no store records the schema that
wrote it. That claim was wrong; it came from the design discussion and was
not checked against the workspace. All four stores already stamp and refuse
forward:

- `longhorn-config` — `SchemaVersion` from `longhorn-core`, refused at
  `store/load.rs:105` as `RecoveryKind::FutureSchema`
- `longhorn-settings` — persists through config domains, so it inherits the
  same check
- `longhorn-history` — structural envelope version refused at
  `persistence.rs:520`, plus an independent payload codec version refused
  separately
- `longhorn-history-tree` — structural version refused at
  `persistence.rs:298`
- backup archives — `UnsupportedFormatVersion` at
  `backup/archive/codec.rs:231`

The non-destructive half holds too: `store/mutation.rs:114` refuses to
mutate a store that loaded as `Recovery`, so a future-schema store is not
overwritten by the next write.

The milestone's gating rationale therefore does not hold, and this card is
no longer a build. What remains is genuinely smaller.

## Remaining Gap

**Coverage.** `longhorn-layout-config` has
`future_schema_and_registry_mismatch_preserve_exact_source`, which is the
shape the whole scenario needs. Nothing asserts the cross-channel case
end-to-end for the other stores: newer store written, older reader opens it,
refuses, and the bytes on disk are unchanged afterwards.

**Classification.** Each store refuses in its own vocabulary —
`RecoveryKind::FutureSchema`, a history structural-version error, a payload
codec error, `UnsupportedFormatVersion`. Contract 018 requires the client
surface to distinguish "this data was written by a newer build" from every
other load failure, and today that means the surface would have to match on
four unrelated error shapes.

## Scope

- cross-channel round-trip tests per store, including bytes-unchanged
- one shared classification for future-schema refusal
- no change to the existing refusal behaviour

## Steps

1. Write the cross-channel test per store, modelled on the layout-config
   test: write under version N+1, open with a reader at N, assert refusal
   and assert the file is byte-identical afterwards.
2. Cover the backup archive path with the same shape.
3. Cover `longhorn-history`'s payload codec version as well as its
   structural version — they refuse independently and both reach a channel
   rejoin.
4. Add one shared classification in `longhorn-core` that each store's
   refusal maps onto, so a caller can ask "was this a future-schema
   refusal?" without knowing which store answered.
5. Do not alter refusal behaviour. This card proves and classifies what
   exists.

## Acceptance Criteria

- every store has a cross-channel refusal test asserting bytes-unchanged
- history's payload codec path is covered independently of its structural
  path
- one classification answers the future-schema question across all stores
- no behaviour change; workspace QA passes
- no crate or package count change

## Evidence Required

- per-store test receipts including the bytes-unchanged assertion
- the shared classification and its mapping from each store's error

## Stop Conditions

- a store turns out to refuse destructively after all, which would make this
  a build card again and re-gate the milestone

## Next Task

Card 151.
