# 004 Configuration Storage, Backup, And Recovery

Status: active  
Owner: Tom  
Updated: 2026-07-28  
Evidence: `../research/translation-memos/002-shared-desktop-systems-follow-up.md`;
`../research/translation-memos/004-configuration-coordination-and-atomic-mutation.md`

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

## Backup

Backup is a registry operation, not a directory copy.

- Domains declare whether they are included, excluded, or require a custom
  backup adapter.
- Every backup has a manifest with app id, app version, creation time, domain
  ids, schema versions, byte sizes, and checksums.
- Backups are written to staging, verified, then published atomically.
- Automatic backups run before migration and may run on a consumer schedule.
- Retention is policy-driven by count and/or age.
- Secrets and caches are excluded by default.
- User-initiated export is distinct from operational backup.

## Restore

- Inspect and checksum an archive before mutation.
- Report compatibility, excluded domains, migrations, and conflicts.
- Restore to staging and validate every included domain.
- Create a safety backup of current state.
- Commit either the complete declared restore set or none of it.
- Emit a machine-readable receipt with restored, skipped, and migrated
  domains.

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
- backup manifests verify and an invalid archive cannot partially restore
- Loophole, Soundcheck, and Bovine state map without sharing product schemas

## Open Decisions

| Decision | First gate |
| --- | --- |
| archive container and compression | `g01.002` backup card |
| encrypted backup adapter | `g01.002` backup card |
| managed-policy precedence | `g01.008` settings composition |
| remote synchronization and conflict behavior | `g01.009` topology adapters |

The initial domain-store card performs read, validation, and in-memory
migration only. It does not require these decisions.
