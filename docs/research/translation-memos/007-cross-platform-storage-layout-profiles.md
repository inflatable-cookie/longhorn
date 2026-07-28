# 007 Cross-platform Storage Layout Profiles

Status: complete and promoted  
Owner: Tom  
Updated: 2026-07-28  
Extends: `002-shared-desktop-systems-follow-up.md`

## Question

How should Longhorn resolve configuration, durable data, machine state,
caches, logs, runtime files, operational backups, and databases across macOS,
Windows, and Linux without forcing every app to assemble paths by hand?

## Sources

Platform authority:

- Apple
  [macOS Library Directory Details](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/MacOSXDirectories/MacOSXDirectories.html)
- freedesktop.org
  [XDG Base Directory Specification 0.8](https://specifications.freedesktop.org/basedir/latest/)
- Microsoft
  [Known Folder IDs](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid)
  and
  [app-data guidance](https://learn.microsoft.com/en-us/windows/apps/develop/data/store-and-retrieve-app-data)
- Tauri
  [`PathResolver`](https://docs.rs/tauri/latest/tauri/path/struct.PathResolver.html)

Donor evidence:

- Loophole `echo-os-paths` resolves separate config, data, cache, state, and
  log directories but uses the product display name on macOS and Windows.
  Aura also carries a hand-written migration from Tauri's bundle-id root.
- Nucleus puts config, SQLite state, snapshots, and drafts below
  `~/.nucleus`, with one explicit root override.
- Soundcheck keeps its SQLite library, settings, geometry, screenshots, and
  integrations below `~/Library/Application Support/Soundcheck`.
- Bovine writes `workspace.json` through Tauri `app_config_dir`.
- Jetstream has a Tauri identifier but no established durable desktop layout.

## Platform Findings

### macOS

Apple assigns app-created durable support data to
`~/Library/Application Support/<bundle-id>` and regenerable data to
`~/Library/Caches/<bundle-id>`. It assigns app logs to `~/Library/Logs`.
Apple says custom code must not create files directly in `Library/Preferences`;
that location belongs behind `NSUserDefaults` or an equivalent system API.

macOS has no general XDG-style custom-file split between configuration, data,
and state. Longhorn should keep those logical classes distinct below
Application Support while still using the native cache and log roots.

Apple recommends the bundle identifier as the leaf. Longhorn follows that
collision-safe convention by default. The canonical id remains
`audio.infiniteloop.soundcheck`; Soundcheck may explicitly register
`Soundcheck` as its stable storage name to preserve a readable existing root.

The storage name is optional and is not read from the current display name. It
is frozen path identity. A product rename does not move data; adding, removing,
or changing the storage name is an explicit transition.

### Linux

XDG defines separate config, data, state, cache, and runtime bases. State is
specifically suitable for layouts, open files, history, and restart state.
Relative XDG environment values are invalid. Runtime storage has session
lifetime, local-filesystem, ownership, and permission requirements; it cannot
silently fall back to a durable home directory.

XDG does not prescribe the application leaf. Longhorn uses canonical app id by
default on every platform. An app may opt into one stable storage name across
the layout instead of maintaining platform-specific leaf overrides.

### Windows

Windows exposes per-user Local and Roaming known folders. Local is appropriate
for durable machine-local settings, state, and larger data. OS roaming data is
no longer supported on Windows 11, and unpackaged apps do not receive the
packaged app-data stores. Longhorn must not imply synchronization by putting
ordinary config below `%APPDATA%`.

The unpackaged default is `%LOCALAPPDATA%/<app-id>` with typed child
directories. An optional stable storage name replaces the app-id leaf for the
whole layout. Packaged adapters may supply their isolated local and local-cache
roots. Remote or cloud settings remain a topology concern, not a path choice.

## Decisions

### Identity

Storage identity is explicit and immutable:

- canonical application id, normally the Tauri/bundle identifier
- optional stable human-readable storage name

The effective leaf is the storage name when supplied, otherwise canonical app
id. The storage name is one validated path component, deliberately chosen, and
then immutable. Display name, executable name, package install directory, and
current working directory are not path inputs. An identity change is a storage
migration.

An invalid explicit storage name fails resolution. It is never ignored in
favor of canonical id. This is an app identity option, not an end-user
preference or per-purpose path override.

### Root purposes

The resolved layout exposes config, data, state, cache, log, runtime, backup,
policy, workspace, and project roots. A database is a storage mechanism, not
a lifecycle class:

- durable app-owned databases live below data, conventionally `databases/`
- machine-local restart-state databases live below state
- rebuildable indexes live below cache
- user-owned document databases use explicit user or project authority

SQLite backup and migration use database-native adapters. A live main/WAL
pair is never moved or copied as ordinary files.

### Built-in profiles

Profile ids are versioned compatibility contracts.

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
replaces it everywhere. It is not the live display name. Resolution reports
the effective leaf and whether it came from canonical or explicit identity.

`unified-app-root-v1` places typed `config/`, `data/`, `state/`, `cache/`,
`logs/`, `runtime/`, and `backups/` children below one native durable app root.
It suits Nucleus-shaped deployments and simple diagnostics. It gives up native
cache cleanup, session-runtime lifetime, and backup classification; resolution
reports those consequences.

`portable-v1` uses the same typed child layout below a required explicit
absolute root. It never guesses the executable directory or current working
directory. Host policy decides whether a packaged or sandboxed app may use it.

Per-purpose overrides remain an explicit deployment and test escape hatch.
They do not create an anonymous profile. Receipts and diagnostics expose every
override and its source.

### Pure resolution

Pure code receives platform directory facts, storage identity, profile id,
and explicit overrides. It returns a complete layout plus provenance,
warnings, and failures. It does not read environment variables, call Tauri,
create directories, or inspect the host filesystem.

A narrow Tauri adapter converts application path API results into platform
facts. Tests use path matrices. Tauri defaults are adapter evidence, not the
profile contract.

### Bootstrap locator

Persisting the selected profile inside a profile-selected config root creates
a bootstrap loop. A tiny versioned locator therefore stays at a fixed
`platform-native-v1` path derived only from canonical app id, never the
optional storage name. It may contain only:

- schema version
- canonical application id
- selected profile id
- explicit portable or unified root when required
- transition id and last committed layout digest

It contains no app settings, secrets, credentials, or arbitrary root map.
Explicit host input may bypass the locator for tests, managed deployments, and
fully portable launches.

An unreadable, corrupt, future, or unknown-profile locator returns recovery
state. It never silently selects another profile and strands an existing
store.

### Profile transition

Changing profile is a storage migration, not an immediate preference write:

1. resolve and display source and target layouts
2. inventory registered ordinary and custom stores
3. reject ambiguous, overlapping, or conflicting targets
4. acquire source and target authorities in deterministic order
5. stage and verify copies; use native database snapshots
6. journal the transition beside the fixed locator
7. commit the locator last
8. retain the source until explicit, receipt-bound cleanup

Cache is rebuildable and excluded by default. Logs are optional evidence.
Runtime material is never migrated. Secrets remain in secure storage.
Interrupted transition recovers to the last committed locator and verified
layout.

Legacy roots are declarative discovery candidates. Discovery is read-only.
Longhorn never automatically merges, deletes, or gives derived display-name
and dot-directory roots authority.

## Rejected Defaults

- `~/.<app>`: pollutes the home root and discards platform lifecycle semantics
- `~/.config/<app>` on macOS: Linux convention, not the Apple app-support
  contract
- directory leaves derived from the live display name: unstable across rename
  and localization
- `%APPDATA%` as generic Windows config: implies obsolete roaming behavior
- one unversioned root enum: profile upgrades could silently relocate data
- selecting a profile from the selected profile's config: bootstrap loop
- live file copy for SQLite: cannot safely capture WAL state

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/package-topology.md`
- `../../contracts/004-configuration-storage-backup-and-recovery.md`
- `../../roadmaps/g01/002-configuration-backup-and-recovery.md`
- cards 011 and 012
