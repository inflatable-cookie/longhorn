# 004 Configuration Storage, Backup, And Recovery

Status: active  
Owner: Tom  
Updated: 2026-08-03
Evidence: `../research/translation-memos/002-shared-desktop-systems-follow-up.md`;
`../research/translation-memos/004-configuration-coordination-and-atomic-mutation.md`;
`../research/translation-memos/005-debounced-mutation-and-explicit-flush.md`;
`../research/translation-memos/006-backup-archive-encryption-and-restore.md`;
`../research/translation-memos/007-cross-platform-storage-layout-profiles.md`

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
| machine state | display, window, device, and machine correlation | platform app state root |
| workspace-local | personal state tied to a workspace | app state root, keyed by workspace |
| project-shared | state intentionally shared with a project/team | explicit project or server authority |
| secret | credentials and private tokens | secure-store adapter |
| cache | recreatable performance data | platform app cache root |
| runtime/log | ephemeral runtime material and logs | runtime/log roots |

The app may expose a portable root explicitly. Longhorn never silently writes
to the current directory or a home-directory dotfile.

## Roots And Paths

- Pure storage code receives platform-directory facts, storage identity,
  profile selection, and explicit overrides through dependency injection.
- Resolution does no filesystem or environment access. It returns a complete
  layout, provenance, warnings, and typed failures.
- Tauri adapters resolve platform-directory facts through the application path
  API. Tauri's suggested leaf layout does not override the selected profile.
- Tests receive temporary roots.
- Filenames are stable, sanitized, and confined below their registered root.
- Ordinary file access opens a capability-scoped root and resolves only the
  validated relative domain path beneath it.
- User-selected project files are not reclassified as app configuration.
- Secrets never share ordinary JSON files or unencrypted backup archives.

## Storage Identity

Every app registers an immutable canonical application id and may register one
optional stable human-readable storage name. The canonical id normally matches
the Tauri and platform bundle identifier.

The effective storage leaf is the storage name when supplied, otherwise the
canonical app id. The storage name is explicitly chosen, validated as one path
component, and never derived from the current display name. Soundcheck may opt
into `Soundcheck` while retaining `audio.example.soundcheck` as canonical
machine identity.

An invalid explicit storage name fails resolution. It never falls back to
canonical id. The option is app identity, not an end-user preference or
per-purpose root override.

Display name, executable name, install path, and working directory never derive
storage paths. Adding, removing, or changing the storage name requires an
explicit transition.

## Layout Profiles

Profile ids are versioned compatibility contracts. Updating Longhorn cannot
change the paths produced by an existing profile id.

`platform-native-v1` is the default:

| Purpose | macOS | Windows | Linux |
| --- | --- | --- | --- |
| config | `Application Support/<leaf>/config` | `LocalAppData/<leaf>/config` | `$XDG_CONFIG_HOME/<leaf>` |
| data | `Application Support/<leaf>/data` | `LocalAppData/<leaf>/data` | `$XDG_DATA_HOME/<leaf>` |
| state | `Application Support/<leaf>/state` | `LocalAppData/<leaf>/state` | `$XDG_STATE_HOME/<leaf>` |
| cache | `Caches/<leaf>` | `LocalAppData/<leaf>/cache` | `$XDG_CACHE_HOME/<leaf>` |
| log | `Logs/<leaf>` | `LocalAppData/<leaf>/logs` | `$XDG_STATE_HOME/<leaf>/logs` |
| runtime | system temp plus `<leaf>` | system temp plus `<leaf>` | `$XDG_RUNTIME_DIR/<leaf>` |
| backup | `Application Support/<leaf>/backups` | `LocalAppData/<leaf>/backups` | `$XDG_DATA_HOME/<leaf>/backups` |

`<leaf>` is canonical app id by default. One optional stable storage name
replaces it throughout the layout. Resolution records the effective leaf plus
canonical or explicit provenance.
Packaged Windows adapters may inject package-isolated local and cache facts,
but may not claim roaming or synchronization.

`unified-app-root-v1` places typed `config`, `data`, `state`, `cache`, `logs`,
`runtime`, and `backups` children below one native durable app root. Resolution
warns that cache cleanup, runtime lifetime, and platform backup classification
are no longer native.

`shared-product-root-v1` places the same typed children below one native
per-user product-data root intended for cooperating product processes:

| Platform | Shared product-data parent | Root with stable name `Loophole` |
| --- | --- | --- |
| macOS | `Application Support` | `Application Support/Loophole` |
| Windows | roaming `AppData` | `%APPDATA%\Loophole` |
| Linux | `$XDG_DATA_HOME` | `$XDG_DATA_HOME/Loophole` |

The host injects this parent as a distinct `shared-data` directory fact.
Windows must not substitute LocalAppData. The profile does not derive a
product leaf from display name and does not normalize its case; canonical id
remains the default leaf unless the app explicitly registers a stable storage
name. Cache, logs, runtime, and backups remain below the durable shared root,
so resolution reports each non-native lifecycle/classification consequence.
This profile supplies a common filesystem authority only. It does not grant
multi-writer ownership or synchronization semantics.

`portable-v1` requires an explicit absolute root and uses the same typed child
layout. It never guesses the executable directory or current directory.

Per-purpose overrides are explicit deployment and test policy. The layout
receipt records each override and its source. Unknown profiles and invalid,
relative, or unavailable required platform facts fail closed.

## Database Placement

A database is a storage mechanism, not a storage class.

- durable app-owned databases use the data root, conventionally below
  `databases`
- machine-local restart-state databases use the state root
- rebuildable indexes use the cache root
- user-owned or project-shared databases use explicit user/project authority

Live databases use native snapshot, migration, and restore adapters. Longhorn
never moves or copies a live SQLite main/WAL pair as ordinary files.

## Profile Bootstrap And Transition

The selected profile cannot be discovered from the root it selects. A minimal
versioned locator stays at a fixed `platform-native-v1` path derived only from
canonical application id, never optional storage name. It contains only
canonical application id, profile id, an explicit profile root when required,
transition id, and the last committed layout digest. It contains no app
configuration, arbitrary per-purpose overrides, or secrets.

Explicit host input may bypass the locator. A missing locator selects the
compiled default. An unreadable, corrupt, future, or unknown-profile locator
returns typed recovery state; it never silently falls back and strands an
existing store.

Profile change is a migration:

1. resolve and preview source and target layouts
2. inventory registered ordinary and custom stores
3. reject overlap, ambiguity, and unresolved destination conflicts
4. acquire source and target authorities in deterministic order
5. stage and verify ordinary copies and native database snapshots
6. durably journal beside the fixed locator
7. commit the locator last
8. retain the old layout until explicit receipt-bound cleanup

Cache is excluded by default, logs are optional evidence, runtime is never
migrated, and secrets remain in secure storage. Crash recovery returns to the
last committed locator and verified layout before normal mutation.

Legacy roots are declared candidates and discovered read-only. Import requires
an explicit plan. Discovery never automatically merges, deletes, or grants
authority to derived display-name, dot-directory, or old Tauri roots.

The implemented transition API inventories both layouts without mutation,
binds confirmation to layout and file evidence, stages ordinary files, invokes
schema-opaque custom adapters under their declared external guards, journals
beside the fixed locator, and commits that locator last. Recovery verifies the
authority selected by the locator. Cleanup accepts only exact registered
source paths and evidence carried by a committed receipt; it re-verifies target
and source state under both store coordinators and remains idempotent.

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

Exporting an inventoried operational archive revalidates its exact digest,
inspects it within the configured archive bounds, and re-encodes the verified
manifest and payload set through the canonical encoder with only the manifest
kind changed to `user-export`. Archive id, creation time, application,
producer, consistency groups, domains, exclusions, payload bytes, and payload
checksums remain unchanged. The operational source is not mutated. A consumer
must not recapture current state, byte-copy an operational archive, or carry a
second ZIP codec to implement export.

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
  authority — supplied since 2026-08-15 by `StoreBackupEncryption`
  (`longhorn-config-age`) over the host's credential store: the operational
  identity is generated once from the OS RNG and kept in the
  `CredentialSlot::AgeIdentity` slot (`longhorn-core`), never in ordinary
  configuration
- interactive export may use age recipients or a human passphrase
- unavailable identity reports `locked`, not `corrupt`
- rotation changes new recipients; external key rings retain old identities
- re-encryption is explicit decrypt-and-write with a new nonce
- retention preserves archives it cannot authenticate and inspect

The age v1 file format is the compatibility contract. A particular Rust age
crate API is replaceable and must preserve the Rust 1.95 floor.

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
transaction authority, and size capabilities. An adapter joins a
failure-atomic set only when it can preserve exact prior state, participate in
the Longhorn journal, and verify rollback. Otherwise it is excluded or
executed as an explicit separate operation with a separate receipt.

SQLite adapters use a database-native snapshot. Longhorn never treats a live
SQLite main/WAL pair as ordinary files.

The implemented adapter seam keeps custom policy in the existing domain
catalogue. Coordinated-bounded capture runs under the Longhorn guard. External
snapshot capture runs only after that guard is released and declares its own
stable group id and transaction authority. Adapter-relative payloads are
confined below `longhorn/adapters/<domain-id>/`, sorted, uniquely declared,
bounded, hashed, and included in capture receipts.

Restore inspection calls the matching adapter without mutation and binds
archive, adapter id, participation, semantic target evidence, and exact current
semantic evidence into a confirmation digest. Custom domains never enter the
ordinary file transaction by inference. The explicit adapter restore call can
require failure atomicity or allow a separately receipted operation. It
reinspects current evidence immediately before execution and rejects terminal
evidence that contradicts the confirmed target or rollback claim.

Adapter semantic evidence is an explicit state: `Absent`, or `Present` with a
SHA-256 digest. It is never an optional target digest. The verified manifest
source state is supplied to inspection. An archived absent custom domain has
zero payloads and must produce an absent target preview; a present custom
domain has one or more payloads and must produce present target evidence.
Contradictory archive, preview, or payload presence rejects the domain without
mutation. Confirmation forms encode `{state: absent}` or
`{state: present, sha256}` so absence cannot collide with a digest or an
omitted field.

Archived absence is admitted only for `GroupedFailureAtomic` adapters. The
existing `Separate` and single-domain `FailureAtomic` protocols remain
present-target-only because their outcome model does not receipt absent
publication or rollback.

Grouped custom restore is a separate explicit capability. It binds one
verified archive, one sorted non-empty domain set, every adapter id, every
per-domain preview, and one group confirmation digest. Every selected adapter
must declare grouped participation and expose the grouped protocol; a
single-domain `FailureAtomic` claim is not promoted into group atomicity.

Grouped execution runs under the store coordinator after the consumer has
quiesced its external authorities:

1. recover or reject any earlier ordinary or grouped restore
2. re-resolve every descriptor and adapter and re-inspect every target
3. reject stale current evidence before staging
4. stage the complete target and exact rollback payload set without live mutation
5. validate size, path, uniqueness, target, and rollback evidence for every stage
6. durably publish the complete private payload set and grouped journal
7. publish each target through its adapter
8. independently verify the complete target set
9. mark success durably, then remove private transaction material

Any failure after journal publication rolls every selected adapter back,
including adapters not yet published, and independently verifies the complete
old evidence set. A crash before the durable journal can leave only disposable
private staging and cannot change live authority. A crash after the journal
causes boot-time recovery to roll the complete group back. Unverified rollback
retains the journal, reports `restore-recovery-required`, and blocks ordinary
loads and mutation.

Longhorn owns the group selection digest, bounds, private target and rollback
payloads, journal phases, ordering, receipts, and restart recovery. Adapters
own schema meaning and the native act of staging, applying, and observing one
domain. Consumers own process quiescence, restart scheduling, and the point at
which the grouped operation runs before live authorities open. Group recovery
requires the exact registered adapter catalogue and never guesses an adapter
from a path or product type.

Both staged states obey one shape rule: absent evidence has zero payloads;
present evidence has at least one payload. The durable journal records explicit
target and rollback evidence independently. Apply and verify requests carry
`Target` or `Rollback` plus the exact expected state. An archived absent target
therefore applies zero target payloads as deletion and verifies `Absent`, while
rollback to an absent prior state applies zero rollback payloads and verifies
`Absent`. These cases remain distinct across restart. Sentinel payloads,
synthetic digests, and inferred absence are forbidden.

Execution and recovery receipts retain each domain's target and rollback
evidence. Receipt projections preserve the explicit state discriminator; they
do not reduce absence to a missing digest. An unsupported journal version or
any journal evidence/payload contradiction remains recovery-required and
blocks normal authority.

The conformance fixture uses SQLite's online backup and restore APIs against a
WAL-mode source, verifies the snapshot, and proves the live WAL is neither
copied as an ordinary payload nor changed by capture. `rusqlite` remains a
test-only dependency; Longhorn owns no SQLite schema or runtime adapter.

## Settings Interaction

Settings UI reads registered domain projections and sends validated patches or
transactions. It never edits files directly. Reset, import, backup, and
restore operate at explicit scopes.

## Acceptance

- config, machine-state, workspace-local, secret, and cache fixtures resolve
  to distinct roots
- all built-in profile/platform matrices resolve deterministically from
  injected facts
- display-name changes do not change a storage layout
- absent storage-name override resolves every app leaf from canonical app id
- one explicit stable storage name replaces every app leaf and appears with
  provenance
- invalid explicit storage name fails instead of falling back to canonical id
- Linux uses XDG bases; unpackaged Windows uses LocalAppData without a roaming
  claim
- unified and portable profiles need no per-purpose override
- invalid or future bootstrap locators cannot silently select another layout
- profile transition commits the locator last and never copies live SQLite as
  ordinary files
- legacy discovery does not mutate either layout
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
- Loophole, Soundcheck, and Split-shell state map without sharing product schemas

## Settings Policy Resolution

Contract 005 resolves the local managed-policy seam. Host authority projects
configured and effective values, policy provenance, constraints, and
editability. Explicit policy overrides or constrains user configuration;
settings cannot mutate a policy-controlled field. Reset removes only the user
override.

## Open Decisions

| Decision | First gate |
| --- | --- |
| remote synchronization and conflict behavior | `g01.009` topology adapters |

The initial domain-store card performs read, validation, and in-memory
migration only. It does not require these decisions.
