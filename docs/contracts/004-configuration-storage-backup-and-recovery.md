# 004 Configuration Storage, Backup, And Recovery

Status: active  
Owner: Tom  
Updated: 2026-07-28  
Evidence: `../research/translation-memos/002-shared-desktop-systems-follow-up.md`;
`../research/translation-memos/004-configuration-coordination-and-atomic-mutation.md`;
`../research/translation-memos/005-debounced-mutation-and-explicit-flush.md`;
`../research/translation-memos/006-backup-archive-encryption-and-restore.md`

## Boundary

Longhorn owns safe, versioned storage mechanisms. The app owns domain schemas,
retention choices, product defaults, and user-facing recovery decisions.

## Storage Classes

Each registered domain declares one class:

| Class | Meaning | Default location |
| --- | --- | --- |
| defaults | compiled, read-only product defaults | package/binary |
| policy | optional administrator or deployment policy | injected read-only root |
| user config | portable user intent and preferences | platform app config root |
| machine state | display, window, device, and machine correlation | platform app data root |
| workspace-local | personal state tied to a workspace | app data root, keyed by workspace |
| project-shared | state intentionally shared with a project/team | explicit project or server authority |
| secret | credentials and private tokens | secure-store adapter |
| cache | recreatable performance data | platform app cache root |
| runtime/log | ephemeral runtime material and logs | temp/log roots |

The app may expose a portable root explicitly. Longhorn never silently writes
to the current directory or a home-directory dotfile.

## Roots And Paths

- Tauri adapters resolve platform roots through the application path API.
- Pure storage code receives resolved roots through dependency injection.
- Tests receive temporary roots.
- Filenames are stable, sanitized, and confined below their registered root.
- Ordinary file access opens a capability-scoped root and resolves only the
  validated relative domain path beneath it.
- User-selected project files are not reclassified as app configuration.
- Secrets never share ordinary JSON files or unencrypted backup archives.

## Domain Files

- A domain has a stable id, schema version, codec, default value, and storage
  class.
- Separate files follow authority and mutation cadence, not individual UI
  controls.
- Unrelated settings, window geometry, layout, cache, and credentials do not
  share one document.
- Cross-domain updates use an explicit transaction/coordinator. A later write
  from one domain cannot overwrite another domain's current state.
- Unknown fields follow the registered codec policy; they are not silently
  discarded by a generic merge.

## Read And Migration

- Missing files yield the domain default and a `missing` diagnostic.
- Invalid files yield a typed recovery state. They are not overwritten during
  load.
- Future schema versions fail safe and preserve the source.
- Older schemas migrate through ordered, testable steps.
- A pre-migration backup is created before the first destructive rewrite.
- Migrations are idempotent at their declared source and target versions.

## Write

- Every ordinary local store has an injected coordination authority. The Tauri
  adapter normally places `.longhorn/config.lock` below the app data root.
- Participating writers acquire a process-local mutex and then an exclusive OS
  advisory lock on that stable file. The lock is store-wide, not per-domain.
- The lock file is never replaced or treated as a stale/PID status file.
- Acquisition uses a finite monotonic deadline and returns typed busy, timeout,
  I/O, or unsupported failures. It never waits forever.
- A mutation rereads the authoritative value after acquisition. It accepts a
  typed patch operation, not a blind whole-document overwrite.
- The non-reentrant guard remains held through patch, validation, encoding,
  publication, durability handling, and receipt creation.
- Serialize and validate before touching the current file.
- Write a uniquely named temporary file in the target directory.
- Use the existing directory capability for exclusive temporary creation and
  same-directory replacement. Do not fall back to an ambient absolute path.
- Sync the file, atomically replace the target, and sync the directory where
  supported.
- Report achieved durability in a typed receipt. A required durability level
  that the platform cannot establish fails instead of being overstated.
- Debounced domains support explicit flush on close and shutdown.
- Failed writes leave the last valid file intact and return a typed error.
- Permissions default to the narrowest practical mode for the data class.

## Coordination Authority

- The default coordinator covers cooperating Longhorn processes on one
  machine and a filesystem with supported local lock semantics.
- Advisory locks do not exclude direct external writers.
- The same coordination authority and stable lock path survive compatible app
  upgrades and coordinate all domains, backup, and restore.
- `project-shared` mutation may use it only for an explicitly single-machine
  local authority. Multi-machine or remote mutation requires a server or other
  external transaction authority.
- Reads do not take the lock. Atomic replacement exposes one complete old or
  new document. Every read-modify-write path rereads under the exclusive
  guard.
- Closing or dropping the owning handle releases the OS lock. A persistent
  unlocked lock file is normal and never needs stale-file cleanup.
- Blocking coordination runs away from the Tauri UI thread.

## Debounced Mutation

Debounce is opt-in. A staged receipt proves process-local acceptance, not
publication. Consumers use immediate mutation when loss of the unflushed
interval after process failure is unacceptable.

- One lane is keyed by store and domain.
- A lane holds one bounded, coalesced typed intent, not a desired whole-domain
  snapshot, unchecked bytes, patch closures, or an unbounded request queue.
- The consumer supplies intent, deterministic coalescer, applicator, pending
  weight, maximum weight, delay, and mutation options.
- Coalescing preserves ordered application to any valid fresh value.
- Longhorn accepts a coalesced candidate only after its weight passes. Rejected
  or overflowing input leaves the earlier intent and deadline unchanged.
- Accepted staging uses trailing-edge timing and monotonic generations.
- Longhorn keeps bounded generation metadata. It does not retain one waiter or
  completion future per stage.

The core owns a deterministic state machine with an injected monotonic clock.
It exposes its next deadline plus due and forced flush calls. It does not own a
thread, async runtime, timer, or Tauri task. Host adapters schedule wakeups and
run blocking flushes away from the UI thread.

Flush composes over coordinated mutation:

1. acquire the card-002 store-wide guard
2. reread the authoritative domain value
3. apply the coalesced intent to that fresh value
4. validate and encode
5. compare current and candidate encoded values
6. skip an equal value or publish the candidate

Separate processes may stage independently. Their flushes serialize at the
coordination guard, and each intent applies to the last published value. Store
loads remain authoritative; pending UI projections do not shadow
`ConfigStore::load`.

## Debounce Failure And Flush

Longhorn performs no hidden retry.

- A failure before atomic replacement retains the exact pending intent and
  enters `retry-required`.
- Due polling does not retry that state. Forced flush retries it; accepted new
  input coalesces with it and starts a new interval.
- An explicit discard operation is the only generic way to remove unpublished
  intent.
- Retry classification distinguishes likely transient failures from
  validation or authority failures, but both retain unpublished intent.
- A failure after known atomic replacement clears intent and reports
  `published-with-durability-failure`. Retrying could apply a non-idempotent
  command twice.

Receipts carry domain id and monotonic generation. Required outcomes distinguish
no pending work, not due, unchanged, published, uncommitted failure with
pending retained, and known publication with durability failure. A terminal
receipt covers all accepted stages through its generation. Lane snapshots
expose pending generation, deadline, retry-required state, and last terminal
result without unbounded history.

Forced aggregate flush visits pending domains in stable domain-id order,
attempts all lanes, and returns one result per lane. It is not a cross-domain
transaction. Only pre-publication failures remain pending.

The host invokes and awaits forced flush before runtime or storage teardown.
Drop and destructors perform no I/O. Shutdown timeout and publication failure
remain visible to the host; a Tauri adapter and product decide whether to
delay, cancel, retry, or continue close.

## Backup

Backup is a registry operation, not a directory copy.

- Every operation names an explicit domain scope. Registry policy marks each
  domain included, excluded with reason, or custom-adapted.
- Secret, cache, runtime, and log domains are excluded by default. Secure-store
  payloads never enter the ordinary archive path.
- Backup captures published state only. A host that needs pending debounced
  intent must force-flush and handle that receipt first.
- Missing files are recorded as absent; defaults are not materialized.
- Readable corrupt or future source may be preserved as non-restorable source
  evidence. An unreadable required source fails capture.

Ordinary capture acquires the existing store-wide guard, inventories the
scope, copies exact bounded source bytes or absence into private immutable
staging, verifies length and SHA-256, then releases the guard. Archive
encoding, compression, encryption, and publication happen after release.

Custom adapters declare coordinated-bounded, external-snapshot, or excluded
capture. Only bounded capture under the Longhorn guard joins its consistency
group. An external transaction such as the SQLite backup API produces a
separate consistency group unless a consumer supplies a higher-level common
authority.

## Archive Format

The portable inner bundle is standard ZIP with media type
`application/vnd.longhorn.config-backup+zip` and extension
`.longhorn-backup`.

Version 1 requires:

- `longhorn/manifest.json` first
- ordinary payloads at `longhorn/domains/<domain-id>.json`
- adapter payloads below `longhorn/adapters/<domain-id>/`
- lexicographic entry order after the manifest
- normalized 1980 ZIP timestamps, `0600` file mode, and no comments or ambient
  filesystem metadata
- DEFLATE writer output; readers accept only Stored and DEFLATE
- regular declared files only; directories, links, devices, duplicates,
  absolute paths, parent traversal, NULs, and undeclared entries are rejected
- finite configurable entry, per-entry byte, total uncompressed byte, and
  compression-ratio limits

The layout is deterministic. Compressed bytes across encoder versions are not
a compatibility promise.

The strict JSON manifest records format, archive id and kind, producer, app
identity, UTC creation time, consistency groups, domain storage and schema
metadata, source state, adapter, payload paths, sizes, SHA-256 checksums, and
explicit exclusions. Unknown format versions or fields fail safe.

SHA-256 proves byte integrity, not authenticity for a plaintext archive.
Inspection and receipts report those states separately.

## Archive Publication And Retention

Operational backup uses an injected app-data backup root. User export uses an
explicit user-selected parent and never enters operational retention.

Both write a unique sibling partial, sync it, reopen and verify it, rename it
once, and report achieved durability. Export does not overwrite an existing
file without explicit caller authority.

Retention acts only on successfully inspected same-app operational archives.
It always preserves the new archive and operation pins. Valid candidates
order by manifest creation time then archive id; age tiers derive from the
newest valid manifest time. Clock regression is diagnosed.

Locked, corrupt, unreadable, foreign-app, unknown-version, and unparseable
files are never deleted automatically. Framework-owned partial cleanup is a
separate bounded policy. Arbitrary files below the backup root are not pruned.

## Backup Encryption

Optional encryption wraps the whole inner ZIP in binary age v1 and uses
extension `.longhorn-backup.age`. ZIP AES and per-entry encryption are not
used. The complete manifest, names, and checksums stay encrypted until an
identity succeeds.

Recipients and identities come from an injected encryption provider.
Longhorn never stores private identities, passphrases, or recovery material in
ordinary configuration.

- automatic encrypted backup requires noninteractive recipient and identity
  authority
- interactive export may use age recipients or a human passphrase
- unavailable identity reports `locked`, not `corrupt`
- rotation changes new recipients; external key rings retain old identities
- re-encryption is explicit decrypt-and-write with a new nonce
- retention preserves archives it cannot authenticate and inspect

The age v1 file format is the compatibility contract. A particular Rust age
crate API is replaceable and must preserve the Rust 1.85 floor.

## Restore

Inspection never mutates the store. It decrypts when required, validates the
strict archive inventory and limits, verifies every payload, and reports app
compatibility, consistency groups, key and authentication state, exclusions,
unknown domains, source and target schemas, migrations, conflicts, and
planned create, replace, delete, migrate, or unchanged actions.

The restore plan binds archive SHA-256, selected actions, explicit conflict
choices, and current present/absent plus SHA-256 evidence into a confirmation
digest. Execution rereads current evidence under the coordinator and rejects
a stale plan.

Archive migrations are side-effect-free during inspection. Execution migrates
and validates the complete selected set into private current-schema staging
before touching live files. Future or incomplete migration blocks that
selected domain. The source archive remains unchanged.

## Restore Transaction And Recovery

Independent domain paths have no portable multi-file atomic rename. Longhorn
promises failure-atomic terminal state, not instantaneous cross-file
visibility.

Execution:

1. verify and stage the complete selected target set
2. acquire the store-wide guard and recheck freshness
3. capture exact current bytes and absence into a private rollback set
4. publish and verify a policy-compliant safety backup
5. durably publish a restore journal with every target, checksum, and phase
6. replace each domain through the existing single-file atomic publisher
7. verify the complete target set
8. mark success durably and clean rollback material

Ordinary failure rolls back and verifies the complete selected set under the
same guard. Unverified rollback retains the journal and enters
`restore-recovery-required`.

A crash leaves the journal and private rollback payloads. The next store open
or mutation completes and verifies rollback before normal writes. Loads that
detect active recovery return typed unavailable state. A load already in
flight may complete against an old or new complete single-domain file.
Callers needing one consistent multi-domain view use a coordinated load-set
operation.

Terminal results are exact:

- success means every selected domain matches staged target evidence
- rolled back means every selected domain matches captured source evidence
- recovery required blocks normal writes until verified rollback

No receipt claims that all lock-free readers observed one cross-domain
generation. Receipts list restored, deleted, unchanged, migrated, excluded,
skipped, rolled-back, and recovery-required domains.

## Custom Backup And Restore Adapters

Adapters declare capture, inspect, stage, publish, verify, rollback,
transaction authority, and size capabilities. An adapter joins the
failure-atomic set only when it can preserve exact prior state, participate in
the journal, and verify rollback. Otherwise it is excluded or executed as an
explicit separate operation with a separate receipt.

SQLite adapters use a database-native snapshot. Longhorn never treats a live
SQLite main/WAL pair as ordinary files.

## Settings Interaction

Settings UI reads registered domain projections and sends validated patches or
transactions. It never edits files directly. Reset, import, backup, and
restore operate at explicit scopes.

## Acceptance

- config, machine-state, workspace-local, secret, and cache fixtures resolve
  to distinct roots
- a killed/failed write preserves the previous valid document
- older schemas migrate after a verified pre-migration backup
- future and corrupt schemas preserve their original bytes
- two concurrent partial updates cannot lose an unrelated field/domain
- two cooperating processes serialize through the stable lock, finite timeout
  is typed, and a crashed holder requires no stale-file deletion
- receipts distinguish atomic replacement from verified durable publication
- debounced intent applies to a fresh locked value and preserves an intervening
  mutation
- fake-clock scheduling proves trailing-edge coalescing without sleeps
- pending weight is bounded and an overflow cannot replace earlier intent
- pre-publication failure retains intent; known post-publication durability
  failure clears it
- close flush reports every lane and never acknowledges a failed write as
  success
- backup manifests verify and an invalid archive cannot partially restore
- archive traversal, duplicates, undeclared entries, links, bombs, unknown
  versions, and locked encryption fail safe
- retention cannot prune the new, pinned, locked, corrupt, foreign, or
  unparseable archive
- restore preview is bound to current evidence and stale confirmation fails
- every publication failure rolls back and verifies or blocks normal writes
- a crash journal recovers before later mutation
- Loophole, Soundcheck, and Bovine state map without sharing product schemas

## Open Decisions

| Decision | First gate |
| --- | --- |
| managed-policy precedence | `g01.008` settings composition |
| remote synchronization and conflict behavior | `g01.009` topology adapters |

The initial domain-store card performs read, validation, and in-memory
migration only. It does not require these decisions.
