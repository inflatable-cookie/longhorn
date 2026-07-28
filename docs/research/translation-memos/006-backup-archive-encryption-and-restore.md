# 006 Backup Archive, Encryption, And Restore

Status: promoted
Owner: Tom
Updated: 2026-07-28
Promotes to: contract 004; g01.002 batches 3 and 4

## Question

How should Longhorn capture, package, retain, inspect, encrypt, and restore a
set of configuration domains without overstating cross-file atomicity or
moving product, secret, and external-database authority into the library?

## Donor Evidence

No donor supplies the complete protocol.

| Donor | Proven behavior | Generic gap |
| --- | --- | --- |
| Soundcheck library | SQLite online backup, integrity check before publish, partial-file cleanup, preserved new/selected backups during pruning, milestone retention, pre-migration and pre-restore backups, post-restore migration, validation, and rollback | one SQLite file; filename metadata is authoritative; no archive manifest, encryption, or crash journal |
| Soundcheck DAW sync | exact-byte content-addressed payloads, absent-file evidence, checksum/length verification, target compatibility, preview-bound confirmation, stale-plan detection, safety snapshot, staged files, post-restore verification, and verified rollback | app-specific host adapters; sequential multi-file replacement has no crash journal or coordination with other processes |
| Loophole Echo | corrupt workspace and command files are renamed to timestamped `.bak` files before defaults are recreated | best-effort rename can fail silently; no manifest, verification, retention, or restore path |
| Loophole Spark | explicit project export and separate autosave/recovery roots | project authority, not reusable configuration backup |
| Nucleus, Jetstream, Bovine | no complete configuration backup/archive protocol found | negative evidence only |

Soundcheck contributes the strongest invariants: preserve exact bytes, record
absence, verify before and after mutation, bind confirmation to a preview,
reject stale plans, preserve current state first, and verify rollback.
Longhorn must not copy its SQLite or DAW schemas.

## Current External Evidence

Primary sources checked on 2026-07-28:

- The [`zip` crate](https://docs.rs/zip/5.1.1/zip/) reads and writes standard
  ZIP, exposes fixed entry metadata and safe-name checks, and has a Rust
  1.83-compatible 5.1.1 release. Current 8.x requires Rust 1.88, above
  Longhorn's Rust 1.85 floor.
- `ZipFile::enclosed_name` rejects absolute and escaping paths, while the
  crate documents that generic extraction is not atomic. Longhorn needs its
  own strict entry inventory and staging.
- The [age v1 format](https://age-encryption.org/v1) is a binary,
  authenticated streaming file format with independently wrapped recipients.
  It supports X25519 and human passphrase recipients without inventing a
  Longhorn cryptographic envelope.
- The Rust [`age` crate](https://docs.rs/age/0.12.1/age/) implements the age v1
  format, supports streaming and multiple recipients, and has a Rust 1.74
  floor. It remains pre-1.0, so the file specification—not its Rust API—is the
  durable contract.
- Rust [`std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html)
  maps one source and destination through the platform rename primitive. It
  offers no multi-path transaction.
- The [SQLite online backup API](https://www.sqlite.org/backup.html) produces
  a consistent database snapshot while allowing incremental capture. This is
  evidence for an external snapshot adapter, not permission to copy a live
  database file.

## Promoted Decision

### One versioned inner bundle

The portable inner format is a standard ZIP file with media type
`application/vnd.longhorn.config-backup+zip`.

Version 1 uses:

- `longhorn/manifest.json` as the first entry
- `longhorn/domains/<domain-id>.json` for ordinary file-backed payloads
- `longhorn/adapters/<domain-id>/<adapter-path>` for custom payloads
- lexicographic entry order after the manifest
- regular files only; no directory, symlink, device, absolute, parent, NUL,
  duplicate, or undeclared entries
- normalized ZIP timestamp `1980-01-01T00:00:00`, Unix mode `0600`, and no
  comments or ambient filesystem metadata
- DEFLATE for non-empty payloads at one writer-selected documented level
- reader support for only Stored and DEFLATE entries
- configurable entry, per-entry byte, total uncompressed byte, and compression
  ratio limits with finite safe defaults

The layout is deterministic. Compressed bytes are not a compatibility
contract across encoder versions. Receipts hash the actual produced bytes.

The file extensions are `.longhorn-backup` for the inner ZIP and
`.longhorn-backup.age` for an encrypted envelope.

### Manifest and integrity

Manifest format version 1 is strict JSON. Unknown format versions, fields,
duplicate ids, invalid domain ids, and undeclared entries fail inspection.
Entries and exclusions are ordered by domain id.

Required archive fields:

- format id and format version
- archive id, kind, UTC creation time, producer version
- app id and app version
- consistency groups and coordination description
- included domain id, storage class, source schema version, source state,
  adapter id, payload paths, byte lengths, and SHA-256 checksums
- explicit absent domains
- explicit exclusions and reasons

SHA-256 detects corruption and binds plans. It does not authenticate an
unencrypted archive. Inspection reports integrity and authenticity
separately.

Missing domains are recorded as absent. Defaults are not materialized.
Readable corrupt or future-version source bytes may be preserved with
`source-preserved` state, but are not silently restorable. An I/O-unreadable
required domain fails capture.

### Scope and consistent capture

Every operation has an explicit domain scope. Registry policy declares each
domain included, excluded with reason, or custom-adapted. Secrets, caches,
runtime files, and logs are excluded by default. Secret adapters never enter
the ordinary archive path.

Backup captures published state only. Pending debounced intent is outside the
store authority. A host requiring it must force-flush first and handle the
flush receipt.

Ordinary configuration capture:

1. acquire the existing store-wide coordination guard
2. inventory the selected registry entries
3. read exact source bytes or absence while holding the guard
4. copy bounded bytes into a private immutable staging snapshot
5. verify staged length and SHA-256
6. release the guard
7. encode, compress, verify, and publish the archive outside the guard

Large streaming domains do not extend the configuration lock indefinitely.
Custom adapters declare one mode:

- `coordinated-bounded`: capture under the Longhorn guard and join its
  consistency group
- `external-snapshot`: use an external transaction authority and return an
  immutable verified snapshot in a separate consistency group
- `excluded`

An external snapshot cannot claim the same instant as ordinary domains unless
the consumer supplies a higher-level authority coordinating both.

### Publication, destinations, and retention

Operational backup uses an injected app-data backup root. User export uses an
explicit user-selected parent and never enters operational retention.
Both stage a unique sibling partial, sync it, verify it by reopening, rename
it once, and report achieved durability. Existing exports are not overwritten
without an explicit caller decision.

Retention acts only on successfully inspected operational archives for the
same app and policy set. It:

- always preserves the archive just published
- preserves pinned, pre-migration, pre-restore, and transaction safety
  archives until their owning operation releases them
- orders valid archives by manifest creation time then archive id
- derives age tiers from the newest valid manifest time, not filesystem mtime
  or the current wall clock
- reports clock regression instead of claiming chronological certainty
- never deletes locked, corrupt, unreadable, foreign-app, unknown-version, or
  unparseable files

Stale framework-owned partials may be cleaned through a separate bounded
cleanup policy. Arbitrary files in the backup directory are never pruned.

### Whole-archive age encryption

Optional encryption wraps the complete inner ZIP in binary age v1. Per-entry
encryption and ZIP AES are not used. Whole-archive encryption hides the
manifest, domain names, and checksums until decryption and gives one
authenticated envelope.

Longhorn receives recipients and identities from an injected encryption
provider. It never stores private identities, recovery phrases, or
passphrases in configuration.

- automatic operational encryption requires a noninteractive recipient and
  identity authority, normally backed by the consumer's secure-store adapter
- interactive export may use age recipient keys or a human passphrase
- passphrases are not retained; passphrase KDF behavior belongs to age
- inspection without a matching identity returns `locked`, not `corrupt`
- key rotation affects new archives; the external key ring retains old
  identities for old archives
- re-encryption is an explicit decrypt-and-write operation with a new nonce,
  never in-place header surgery
- retention preserves an archive when its manifest cannot be authenticated
  and inspected

The age file format is the compatibility promise. A Rust implementation crate
is replaceable and must remain compatible with Rust 1.85.

### Inspect and plan before restore

Inspection never mutates the store. It decrypts when required, validates the
ZIP inventory and limits, verifies every payload, checks app identity and
domain registration, and reports:

- compatible, migratable, future, corrupt, absent, excluded, unknown, and
  adapter-unavailable domains
- selected restore set and explicit skips
- source and target schema versions
- planned create, replace, delete, migrate, or unchanged actions
- integrity, authentication, consistency-group, and key state

A restore plan binds the archive SHA-256, selected actions, current
present/absent plus SHA-256 evidence, and explicit conflict choices into a
confirmation digest. Execution rereads current evidence under the coordinator
and rejects a stale plan.

Archive migration is side-effect-free during inspect. Execution migrates and
validates payloads into a private current-schema staging set before touching
live files. The archive remains unchanged. Future or incomplete migration
blocks that selected domain.

### Recoverable restore transaction

Portable filesystems do not offer one atomic rename for several independent
paths. Longhorn therefore promises failure-atomic terminal state, not
instantaneous cross-file visibility.

Execution:

1. verify and stage the complete selected target set
2. acquire the store-wide coordination guard and recheck plan freshness
3. capture exact current bytes and absence into a private rollback set
4. publish and verify a policy-compliant safety backup
5. durably publish a restore journal naming every target, old/new checksum,
   and phase
6. replace each domain through the existing single-file atomic publisher
7. verify the complete target set
8. mark success durably, remove rollback material, then release safety pins

Any ordinary failure rolls back every selected domain under the same guard
and verifies the rollback before returning. If rollback cannot be verified,
the journal and rollback material remain and the store enters
`restore-recovery-required`.

A crash leaves the durable journal. The next store open or mutation must
complete and verify rollback before normal writes. Loads that detect an active
or recovery-required journal return typed unavailable state. A load already
in flight may finish against an old or new complete single-domain file.
Callers requiring a consistent multi-domain view use a coordinated load-set
operation.

The terminal guarantees are:

- success: every selected domain matches the staged target
- reported rolled back: every selected domain matches the captured source
- recovery required: normal writes remain blocked until verified rollback

No receipt claims a moment when all lock-free readers observed one generation.

### Custom restore adapters

An adapter declares capture, inspect, stage, publish, verify, and rollback
capabilities plus its transaction authority and size limits. It joins the
failure-atomic restore set only when it can preserve exact prior state,
participate in the operation journal, and verify rollback. Otherwise it is
excluded or executed as an explicit separate operation with a separate
receipt.

SQLite adapters use the database backup API or another database-native
snapshot mechanism. Longhorn never copies a live SQLite main/WAL pair as
ordinary files.

## Rejected Options

| Option | Reason |
| --- | --- |
| directory copy | ignores registry policy, schemas, secrets, and live database semantics |
| tar plus zstd | weaker desktop interoperability and more path/metadata surface for small config bundles |
| custom binary container | unnecessary format and tooling burden |
| ZIP AES or per-entry encryption | exposes archive inventory and creates mixed encryption states |
| custom XChaCha/Argon envelope | invents a cryptographic file format and recovery policy |
| filesystem mtime retention | mutable, lossy, and unreliable across copies and clock changes |
| prune corrupt or locked files | can destroy the only recoverable copy |
| hold the config lock through compression/encryption | extends writer exclusion beyond the consistent-cut requirement |
| stream unbounded domains under the config lock | makes lock duration unbounded |
| copy live SQLite files | can miss WAL state and bypass database consistency authority |
| claim multi-file atomic rename | no portable primitive supports it |
| resume a crashed restore by default | completes a destructive choice after confirmation context was lost |
| rollback from an encrypted archive only | key loss could make local crash recovery impossible |
| materialize defaults for missing files | changes absence into persisted state |

## Required Proof

- strict version-1 manifest and exact entry inventory
- deterministic paths, order, timestamps, modes, and writer configuration
- Stored/DEFLATE interoperability plus ZIP bomb, duplicate, traversal,
  symlink, undeclared-entry, and size-limit rejection
- SHA-256 verification and separate integrity/authentication reporting
- bounded under-lock snapshot with concurrent writer exclusion
- absent, migrated, future, corrupt-source, unreadable, and excluded fixtures
- operational versus export destination and overwrite behavior
- safe retention with clock regression, corrupt manifests, locked encryption,
  pins, and newly published preservation
- age recipient and passphrase round trips with unavailable and rotated keys
- inspect-only behavior, compatibility report, confirmation binding, and
  stale-plan rejection
- pre-migration and pre-restore safety backup
- injected failure at every restore publication point with verified rollback
- crash-journal recovery and write refusal until recovery
- custom adapter refusal when exact rollback cannot be proven
- SQLite external-snapshot fixture
- Rust 1.85 and host/UI-free core package graph

## Promotion Result

Contract 004 now owns a standard ZIP inner bundle, strict checksummed
manifest, bounded coordinated capture, safe retention, whole-archive age
encryption, inspect-bound restore planning, and journaled failure-atomic
restore.

The implementation is split across cards 005 through 010. Card 005 is ready.
