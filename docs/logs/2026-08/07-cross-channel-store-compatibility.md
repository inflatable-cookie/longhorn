# Cross-channel Store Compatibility Proof And Classification

Date: 2026-08-07
Card: 150
Roadmap: g02.009

## Result

Every persistent store already refused a future schema before this card;
none of that behaviour changed. What the card added is the proof that the
refusal is also non-destructive, and one shared classification so the update
surface can recognise a channel rejoin without matching per-store errors.

## Premise Correction

The card was compiled on the claim that no store records the schema that
wrote it, which came from the design discussion and was never checked
against the workspace. It was false in every store:

| Surface | Refusal site |
| --- | --- |
| configuration | `store/load.rs:105`, `RecoveryKind::FutureSchema` |
| settings | inherits configuration by storing through a config domain |
| history | `persistence.rs:520` structural, plus payload codec |
| history tree | `persistence.rs:298` structural, plus payload codec |
| backup archive | `backup/archive/codec.rs:231` |

`store/mutation.rs:114` already refused to mutate a store that loaded as
`Recovery`, so the destructive write-back was not reachable either. Memo 019
carries the correction; the milestone lost its gate as a result.

## Shape

- `longhorn-core::store_compatibility` — `CompatibilityStore`,
  `FutureSchemaRefusal`, and the `FutureSchemaRefused` trait. Versions are
  `Option<u32>` because configuration reports them only as prose in
  `RecoveryState::detail`; parsing that back would be worse than reporting
  neither. History, history tree, and archives all report both numbers.
- Implementations on `RecoveryState`, `HistoryLoadError`, `ForkLoadError`,
  and `BackupArchiveError`. No existing type changed shape, so no consumer
  break.
- `BACKUP_FORMAT_VERSION` widened from private to `pub(crate)`.

## Evidence

- `longhorn-config` `cross_channel` — four tests: refused load leaves the
  file byte-identical, refused *mutation* leaves it byte-identical,
  classification, and a corrupt document that must not classify as a version
  problem.
- `longhorn-settings-config` `cross_channel` — proves the inheritance rather
  than assuming it, including that settings classify as `Configuration`
  because that is what they store through.
- `longhorn-history` `future_versions_classify_for_the_update_surface` —
  both version axes with their numbers, plus a structural fault that must
  not classify.
- `longhorn-history-tree` `future_versions_classify_for_the_update_surface`
  — same shape. Writing it caught that the first implementation mapped only
  the structural variant and missed `FuturePayloadCodecVersion`.
- Full round: `cargo fmt --check` clean, clippy clean on both the default
  and `--all-features` passes, `cargo test --workspace --locked` 149 suites
  green with zero failures.

## Notes

The mutation test is the one that would have caught a real regression. The
dangerous path in a channel rejoin is not the refused read — it is a reader
that falls back to defaults and then persists those defaults over newer
data. That path is closed, and now proved closed.

History and history tree decode from caller-supplied bytes rather than
owning a file, so "bytes unchanged" is not a meaningful assertion there;
their existing tests already assert the live state is untouched
(`assert_eq!(live, before)`). The file-level proof is asserted where a file
exists: configuration, settings, and archives.
